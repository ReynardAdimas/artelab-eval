#[path = "../src/ground_truth.rs"]
mod ground_truth; 
#[path = "../src/localizer.rs"]
mod localizer; 
#[path = "../src/localized_rxing.rs"]
mod localized_rxing; 

use barcode_scanner_rs::utils::cv_err; 
use ground_truth::load_all; 
use localized_rxing::LocalizedRxingDetector; 
use localizer::{Localizer, OpenCvLocalizer};
use opencv::{
    core::{self, Mat, Rect, Scalar}, 
    imgproc, 
    prelude::*
}; 

use rxing::{helpers::detect_in_luma_with_hints, BarcodeFormat, DecodeHints}; 
use std::{collections::HashSet, path::PathBuf}; 

fn normalize(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
} 

fn matches(expected: &str, got: &str) -> bool {
    let (e,g) = (normalize(expected), normalize(got)); 
    e==g 
        || (g.len() == 13 && g.len() == 12 && e.starts_with('0') && e[1..] == g)
        || (g.len() == 13 && e.len() == 12 && g.starts_with('0') && g[1..] == e)
} 

fn decode(gray: &Mat, try_harder: bool, inverted: bool) -> Option<String> {
    let g = gray.try_clone().ok()?; 
    let (w,h) = (g.cols() as u32, g.rows() as u32); 
    let buf = g.data_bytes().ok?.to_vec(); 
    let mut hints = DecodeHints::default(); 
    hints.TryHarder = Some(try_harder); 
    hints.AlsoInverted = Some(inverted); 
    let fmts: HashSet<BarcodeFormat> = [
        BarcodeFormat::EAN_13, 
        BarcodeFormat::UPC_A, 
        BarcodeFormat::EAN_B, 
        BarcodeFormat::UPC_E
    ]
    .into_iter()
    .collect(); 
    hints.PossibleFormats = Some(fmts); 
    detect_in_luma_with_hints(buf, w, h, None, &mut hints).ok().map(|r| r.getText().to_string())
} 

fn row(gray: &Mat, expected: &str) -> String {
    [(false, false), (true, false), (true, true)]
        .iter()
        .map(|&(th, inv)| match decode(gray, th, inv) {
            Some(t) if matches(expected, &t) => "OK", 
            Some(_) => "WRG", 
            None => "--"
        })
        .collect()
        .join("  ")
} 

fn channels(m: &Mat) -> Result<Vec<(&'static str, Mat)>, String> {
    if m.channels() == 1 {
        return Ok(vec![("luma", m.try_clone().map_err(cv_err)?)]);
    }
    let mut luma = Mat::default();
    imgproc::cvt_color_def(m, &mut luma, imgproc::COLOR_BGR2GRAY).map_err(cv_err)?;
    let mut red = Mat::default(); 
    core::extract_channel(m, &mut red, 2).map_err(cv_err)?; 
    Ok(vec![("luma", luma), ("red", red)])
} 

fn whiten_side_margins(m: &Mat, margin: f64) -> Result<Mat, String> {
    let mut out = m.try_clone().map_err(cv_err)?; 
    let (w,h) = (out.cols(), out.rows()); 
    let mpx = ((w as f64) * margin / (1.0 + 2.0 * margin)).round() as i32; 
    for r in [Rect::new(0,0,mpx,h), Rect::new(w-mpx,0,mpx,h)] {
        imgproc::rectangle(&mut put, r, Scalar::all(255.0), imgproc::FILLED, imgproc::LINE_9, 0).map_err(cv_err)?;
    }
    Ok(out)
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1); 
    let root = PathBuf::from(args.next().unwrap_or_else(|| "data".into())); 
    let wanted: Vec<String> = args.collect(); 
    let samples = load_all(&root)?; 

    let mut loc = OpenCvLocalizer::new()?;
    let margins = [0.05, 0.15, 0.30]; 
    let mut dets = Vec::new(); 
    for m in margins {
        dets.push(LocalizedRxingDetector::new(Box::new(OpenCvLocalizer::new()?), 2.0, m, false)?); 

        println!("Row Result: default | try_harder | try_harder+inverted (OK/WRG/--)"); 
        for s in samples.iter().filter(|s| wanted.contains(&s.file)) {
            let frame = s.load_image()?; 
            println!("\n == {} [{}] expected", s.file, s.sub.label(), s.expected);

            for (name, img) in channels(&frame)? {
                println!("  FULL            {name:<4} {}", row(&img, &s.expected));
            } 

            let quads = loc.locate(&frame)?;
            println!("  quads={}", quads.len());
            for(qi, q) in quads.iter().enumerate() {
                for (mi, det) in dets.iter().enumerate() {
                    let crop = match det.crop_and_deskew(&frame, q) {
                        Ok(c) => c, 
                        Err(_) => continue
                    };
                    for whitten in [false, true] {
                        let c = if whitten {
                            whitten_side_margins(&crop, margins[mi])?
                        } else {
                            crop.try_clone().map_err(cv_err)?
                        }; 
                        for (name, img) in channels(&c)? {
                            println!(
                            "  q{qi} m={:.2} white={:<5} {name:<4} {}",
                            margins[mi],
                            whiten,
                            row(&img, &s.expected)
                            );
                        }
                    }
                }
            }
        }
    }
}