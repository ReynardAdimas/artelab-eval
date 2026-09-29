use barcode_scanner_rs::{
    detector::{BarcodeDetector, PipelineDetector, RxingDetector, StandardQrDetector, WeChatDetector},
    model::ModelPaths, 
    preprocess::{PreprocessKind, Preprocessor}
}; 
use std::{collections::BTreeMap, path::PathBuf, time::Instant};

mod ground_truth;
use ground_truth::{load_all, SubDataset}; 

fn normalize(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

#[derive(Default)]
struct Stat {
    n: usize, 
    ok: usize, 
    miss: usize, 
    err: usize, 
    wrong: usize, 
    ms: Vec<f64>
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    } 
    sorted[((sorted.len() - 1)as f64 * p).round() as usize]
}

fn main() -> Result<(), String>{
    let data_root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "data".to_string())); 
    let samples = load_all(&data_root)?; 

    let combos: Vec<(&str, PreprocessKind, bool)> = vec![
                ("raw+rxing", PreprocessKind::Raw, false),
        ("gray+rxing", PreprocessKind::Gray, false),
        ("gray_otsu+rxing", PreprocessKind::GrayOtsu, false),
        ("gray_otsu+hybrid", PreprocessKind::GrayOtsu, true),
    ]; 

    let mut stats: BTreeMap<(SubDataset, &str), Stat> = BTreeMap::new(); 

    for (label, kind, use_hybrid) in &combos {
        let mut pre = Preprocessor::new(*kind)?;

        let mut hybrid: Option<PipelineDetector> = if *use_hybrid {
            Some(PipelineDetector::new(vec![
                Box::new(StandardQrDetector::new()?), 
                Box::new(WeChatDetector::new(&ModelPaths::wechat_default())?), 
                Box::new(RxingDetector::new())
            ]))
        } else {
            None
        }; 

        let mut rxing_only = RxingDetector::new(); 

        for s in &samples {
            let img = pre.run(&s.img)?;
            let t0 = Instant::now(); 
            let res = match &mut hybrid {
                Some(h) => h.detect(&img), 
                None => rxing_only.detect(&img)
            }; 
            let dt = t0.elapsed().as_secs_f64(); 

            let key = (s.sub, *label); 
            let st = stats.entry(key).or_default();
            st.n += 1;
            st.ms.push(dt); 

            match res {
                Err(_) => st.err += 1,
                Ok(r) if r.is_empty() => st.miss += 1, 
                Ok(r) => {
                    let expected = normalize(&s.expected); 
                    let got_norm: Vec<String> = r.iter().map(|d| normalize(&d.data)).collect();
                    if got_norm.iter().any(|g| *g == expected) {
                        st.ok += 1;
                    } else {
                        st.wrong += 1;
                    }
                }
            }
        }
    }

    println!(
        "\n{:<14} {:<18} {:>4} {:>7} {:>7} {:>7} {:>7} {:>9} {:>9}",
        "sub-dataset", "combo", "n", "ok%", "miss%", "err%", "wrong%", "p50(ms)", "p95(ms)"
    );
    for ((sub, label), st) in stats.iter_mut() {
        st.ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p = |x: usize| 100.0 * x as f64 / st.n as f64;
        println!(
            "{:<14} {:<18} {:>4} {:>6.1}% {:>6.1}% {:>6.1}% {:>6.1}% {:>9.2} {:>9.2}",
            sub.label(), label, st.n, p(st.ok), p(st.miss), p(st.err), p(st.wrong),
            percentile(&st.ms, 0.5), percentile(&st.ms, 0.95),
        );
    }

    Ok(())
}
