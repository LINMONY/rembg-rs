#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod gui;
mod models;

use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use image::imageops::{self, FilterType};
use image::{DynamicImage, GrayImage, Luma, Rgba, RgbaImage};
use models::{ModelSpec, PostProcessMode, MODELS};
use tract_onnx::prelude::*;

type RunnableModel = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

pub struct RembgEngine {
    pub spec: ModelSpec,
    pub model: RunnableModel,
}

impl RembgEngine {
    pub fn load<F>(
        spec: ModelSpec,
        cancel_flag: &AtomicBool,
        on_progress: F,
    ) -> Result<Self, Box<dyn std::error::Error>>
    where
        F: FnMut(u64, u64),
    {
        let model_path = ensure_model_spec(&spec, cancel_flag, on_progress)?;
        if cancel_flag.load(Ordering::Relaxed) {
            return Err("Cancelled".into());
        }
        let model = build_runnable_model(&model_path, spec.input_w, spec.input_h)?;
        Ok(Self { spec, model })
    }

    pub fn remove_background_image(
        &self,
        img: &DynamicImage,
    ) -> Result<RgbaImage, Box<dyn std::error::Error>> {
        let mask = self.predict_mask(img)?;
        Ok(naive_cutout(img, &mask))
    }

    fn predict_mask(&self, img: &DynamicImage) -> Result<GrayImage, Box<dyn std::error::Error>> {
        let w = self.spec.input_w;
        let h = self.spec.input_h;
        let input_tensor = normalize_image(img, w, h, self.spec.mean, self.spec.std);
        let outputs = self.model.run(tvec!(input_tensor.into()))?;

        let pred_view = outputs[0].to_array_view::<f32>()?;
        let raw_slice = pred_view
            .as_slice()
            .ok_or("Output tensor is not contiguous")?;
        let hw = (w * h) as usize;

        let mut mask_small = GrayImage::new(w, h);

        match self.spec.postprocess {
            PostProcessMode::ClothArgmax => {
                let channels = (raw_slice.len() / hw).max(1);
                for y in 0..h {
                    for x in 0..w {
                        let idx = (y * w + x) as usize;
                        let mut best_c = 0usize;
                        let mut best_v = f32::NEG_INFINITY;
                        for c in 0..channels {
                            let v = raw_slice[c * hw + idx];
                            if v > best_v {
                                best_v = v;
                                best_c = c;
                            }
                        }
                        let byte = if best_c > 0 { 255u8 } else { 0u8 };
                        mask_small.put_pixel(x, y, Luma([byte]));
                    }
                }
            }
            PostProcessMode::DirectMatte => {
                let slice = &raw_slice[..hw.min(raw_slice.len())];
                for y in 0..h {
                    for x in 0..w {
                        let v = slice[(y * w + x) as usize].clamp(0.0, 1.0);
                        mask_small.put_pixel(x, y, Luma([(v * 255.0) as u8]));
                    }
                }
            }
            PostProcessMode::MinMax | PostProcessMode::SigmoidMinMax => {
                let slice = &raw_slice[..hw.min(raw_slice.len())];
                let apply_sigmoid = self.spec.postprocess == PostProcessMode::SigmoidMinMax;

                let mut vals = vec![0.0f32; slice.len()];
                let mut ma = f32::NEG_INFINITY;
                let mut mi = f32::INFINITY;

                for (i, &raw_v) in slice.iter().enumerate() {
                    let v = if apply_sigmoid {
                        1.0 / (1.0 + (-raw_v).exp())
                    } else {
                        raw_v
                    };
                    vals[i] = v;
                    if v > ma {
                        ma = v;
                    }
                    if v < mi {
                        mi = v;
                    }
                }

                let range = (ma - mi).max(1e-6);
                for y in 0..h {
                    for x in 0..w {
                        let v = vals[(y * w + x) as usize];
                        let norm = ((v - mi) / range).clamp(0.0, 1.0);
                        mask_small.put_pixel(x, y, Luma([(norm * 255.0) as u8]));
                    }
                }
            }
        }

        let (orig_w, orig_h) = (img.width(), img.height());
        let filter = if self.spec.postprocess == PostProcessMode::ClothArgmax {
            FilterType::Nearest
        } else {
            FilterType::Lanczos3
        };
        let mask_resized = imageops::resize(&mask_small, orig_w, orig_h, filter);
        Ok(mask_resized)
    }
}

pub fn ensure_model_spec<F>(
    spec: &ModelSpec,
    cancel_flag: &AtomicBool,
    mut on_progress: F,
) -> Result<PathBuf, Box<dyn std::error::Error>>
where
    F: FnMut(u64, u64),
{
    let target_path = spec.local_path();
    if target_path.exists() && target_path.metadata()?.len() > 1_000_000 {
        return Ok(target_path);
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let response = ureq::get(spec.url).call()?;
    let total_size = response
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or((spec.size_mb as u64) * 1_048_576);

    let tmp_path = target_path.with_extension("onnx.part");
    let mut reader = response.into_reader();
    let mut file = BufWriter::new(File::create(&tmp_path)?);

    let mut buffer = [0u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    let mut last_report: u64 = 0;

    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            drop(file);
            let _ = fs::remove_file(&tmp_path);
            return Err("Cancelled".into());
        }

        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;

        if downloaded - last_report >= 512 * 1024 || downloaded == total_size {
            last_report = downloaded;
            on_progress(downloaded, total_size);
        }
    }
    file.flush()?;
    drop(file);

    fs::rename(&tmp_path, &target_path)?;
    Ok(target_path)
}

pub fn build_runnable_model(
    model_path: &Path,
    input_w: u32,
    input_h: u32,
) -> Result<RunnableModel, Box<dyn std::error::Error>> {
    let onnx = tract_onnx::onnx();
    let mut proto = onnx.proto_model_for_path(model_path)?;
    if let Some(graph) = proto.graph.as_mut() {
        let mut extra_nodes = Vec::new();
        for (idx, node) in graph.node.iter_mut().enumerate() {
            if node.op_type == "Resize" {
                for attr in &mut node.attribute {
                    if attr.name == "coordinate_transformation_mode"
                        && (attr.s.as_slice() == b"pytorch_half_pixel"
                            || attr.s.as_slice() == b"tf_half_pixel_for_nn")
                    {
                        attr.s = b"half_pixel".to_vec();
                    }
                }

                use tract_onnx::pb::{
                    attribute_proto::AttributeType, tensor_proto::DataType, AttributeProto,
                    NodeProto, TensorProto,
                };

                let mut pre_nodes = Vec::new();

                if node.input.len() >= 2 && node.input[1].is_empty() {
                    let roi_name = format!("__resize_roi_{}", idx);
                    pre_nodes.push(NodeProto {
                        name: format!("{}_const", roi_name),
                        op_type: "Constant".to_string(),
                        output: vec![roi_name.clone()],
                        attribute: vec![AttributeProto {
                            name: "value".to_string(),
                            r#type: AttributeType::Tensor as i32,
                            t: Some(TensorProto {
                                dims: vec![0],
                                data_type: DataType::Float as i32,
                                ..Default::default()
                            }),
                            ..Default::default()
                        }],
                        ..Default::default()
                    });
                    node.input[1] = roi_name;
                }

                if node.input.len() == 4 {
                    if !node.input[3].is_empty() {
                        let x_in = node.input[0].clone();
                        let sizes_in = node.input[3].clone();
                        let prefix = format!("__resize_fix_{}", idx);
                        let shape_out = format!("{}_shape", prefix);
                        let shape_f32 = format!("{}_shape_f32", prefix);
                        let sizes_f32 = format!("{}_sizes_f32", prefix);
                        let scales_f32 = format!("{}_scales_f32", prefix);

                        let make_cast_f32 =
                            |name: String, input: String, output: String| NodeProto {
                                name,
                                op_type: "Cast".to_string(),
                                input: vec![input],
                                output: vec![output],
                                attribute: vec![AttributeProto {
                                    name: "to".to_string(),
                                    r#type: AttributeType::Int as i32,
                                    i: 1,
                                    ..Default::default()
                                }],
                                ..Default::default()
                            };

                        pre_nodes.push(NodeProto {
                            name: format!("{}_shape_op", prefix),
                            op_type: "Shape".to_string(),
                            input: vec![x_in],
                            output: vec![shape_out.clone()],
                            ..Default::default()
                        });
                        pre_nodes.push(make_cast_f32(
                            format!("{}_cast_shape", prefix),
                            shape_out,
                            shape_f32.clone(),
                        ));
                        pre_nodes.push(make_cast_f32(
                            format!("{}_cast_sizes", prefix),
                            sizes_in,
                            sizes_f32.clone(),
                        ));
                        pre_nodes.push(NodeProto {
                            name: format!("{}_div_scales", prefix),
                            op_type: "Div".to_string(),
                            input: vec![sizes_f32, shape_f32],
                            output: vec![scales_f32.clone()],
                            ..Default::default()
                        });

                        node.input[2] = scales_f32;
                    }
                    node.input.truncate(3);
                }

                if !pre_nodes.is_empty() {
                    extra_nodes.push((idx, pre_nodes));
                }
            }
        }
        for (idx, nodes_to_insert) in extra_nodes.into_iter().rev() {
            for (offset, n) in nodes_to_insert.into_iter().enumerate() {
                graph.node.insert(idx + offset, n);
            }
        }
    }

    let model = onnx
        .model_for_proto_model(&proto)?
        .with_input_fact(
            0,
            InferenceFact::dt_shape(
                f32::datum_type(),
                tvec!(1, 3, input_h as usize, input_w as usize),
            ),
        )?
        .into_optimized()?
        .into_runnable()?;

    Ok(model)
}

fn normalize_image(
    img: &DynamicImage,
    input_w: u32,
    input_h: u32,
    mean: [f32; 3],
    std: [f32; 3],
) -> Tensor {
    let rgb = img.to_rgb8();
    let resized = imageops::resize(&rgb, input_w, input_h, FilterType::Lanczos3);

    let max_pixel = resized.as_raw().iter().copied().max().unwrap_or(0) as f32;
    let denom = max_pixel.max(1e-6);

    let hw = (input_w * input_h) as usize;
    let mut data = vec![0.0f32; 3 * hw];

    for y in 0..input_h {
        for x in 0..input_w {
            let pixel = resized.get_pixel(x, y);
            let idx = (y * input_w + x) as usize;
            for c in 0..3 {
                let val = (pixel[c] as f32) / denom;
                data[c * hw + idx] = (val - mean[c]) / std[c];
            }
        }
    }

    tract_ndarray::Array4::from_shape_vec((1, 3, input_h as usize, input_w as usize), data)
        .expect("Invalid tensor shape")
        .into()
}

fn naive_cutout(img: &DynamicImage, mask: &GrayImage) -> RgbaImage {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut out = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let p = rgba.get_pixel(x, y);
            let m = mask.get_pixel(x, y)[0] as u32;
            let r = ((p[0] as u32 * m + 127) / 255) as u8;
            let g = ((p[1] as u32 * m + 127) / 255) as u8;
            let b = ((p[2] as u32 * m + 127) / 255) as u8;
            let a = ((p[3] as u32 * m + 127) / 255) as u8;
            out.put_pixel(x, y, Rgba([r, g, b, a]));
        }
    }

    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let input_path = PathBuf::from(&args[1]);
        let output_path = if args.len() > 2 {
            PathBuf::from(&args[2])
        } else {
            let stem = input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            input_path.with_file_name(format!("{}_nobg.png", stem))
        };
        let model_id = if args.len() > 3 {
            args[3].as_str()
        } else {
            "u2net"
        };
        let spec = MODELS
            .iter()
            .copied()
            .find(|m| m.id == model_id)
            .unwrap_or(MODELS[0]);

        let cancel = AtomicBool::new(false);
        let engine = RembgEngine::load(spec, &cancel, |_, _| {})?;
        let img = image::open(&input_path)?;
        let cutout = engine.remove_background_image(&img)?;
        cutout.save(&output_path)?;
        return Ok(());
    }

    gui::run_gui()
}
