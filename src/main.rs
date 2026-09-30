use barcode_scanner_rs::{
    detector::{BarcodeDetector, PipelineDetector, RxingDetector, StandardQrDetector, WeChatDetector},
    model::ModelPaths, 
    preprocess::{PreprocessKind, Preprocessor}
};
use std::{collections::BTreeMap, path::PathBuf, time::Instant};

mod ground_truth;
use ground_truth::{load_all, SubDataset, load_muenster, load_deal_kaist, }; 
mod localized_rxing;
use localized_rxing::LocalizedRxingDetector;

fn normalize(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

enum DecoderKind {
    LocalizedRxing { local_otsu: bool }
} 



#[derive(Default)]
struct Stat {
    n: usize, 
    total_returned: usize, 
    correct_returns: usize,
    correct_barcodes: usize,
    err: usize, 
    ms: Vec<f64>
} 

impl Stat {
    fn reading_rate(&self) -> f64 {
        100.0 * self.correct_barcodes as f64 / self.n as f64
    } 
    fn precision(&self) -> f64 {
        if self.total_returned == 0 {
            return f64::NAN;
        }
        100.0 * self.correct_returns as f64 / self.total_returned as f64
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    } 
    sorted[((sorted.len() - 1)as f64 * p).round() as usize]
}

fn matches(expected: &str, got: &str) -> bool {
    let e = normalize(expected); 
    let g = normalize(got);
    if e == g {
        return true;
    } 
    if e.len() == 13 && g.len() == 12 && e.starts_with('0') && e[1..] == g {
        return true;
    } 
    if g.len() == 13 && e.len() == 12 && g.starts_with('0') && g[1..] == e {
        return true;
    }
    false
}

fn build_decoder(kind: &DecoderKind) -> Result<Box<dyn BarcodeDetector>, String> {
    Ok(match kind {
        DecoderKind::LocalizedRxing { local_otsu } => {
            Box::new(LocalizedRxingDetector::new(2.0, 0.15, *local_otsu)?)
        }
    })
}

fn main() -> Result<(), String>{
    let data_root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "data".to_string())); 
    let samples = load_deal_kaist(&data_root)?; 

    let combos: Vec<(&str, PreprocessKind, DecoderKind)> = vec![
        ("raw+localize+rxing", PreprocessKind::Raw, DecoderKind::LocalizedRxing { local_otsu: false }),
        ("gray+localize+rxing", PreprocessKind::Gray, DecoderKind::LocalizedRxing { local_otsu: false }),
        ("gray+localize_otsu+rxing", PreprocessKind::Gray, DecoderKind::LocalizedRxing { local_otsu: true }),
    ]; 

    let mut stats: BTreeMap<(SubDataset, &str), Stat> = BTreeMap::new(); 

    for (label, kind, decoder_kind) in &combos {
        let mut pre = Preprocessor::new(*kind)?;
        let mut decoder  = build_decoder(decoder_kind)?;
        let total  = samples.len();


        for (idx,s) in samples.iter().enumerate() {
            let raw = match s.load_image() {
                Ok(m) => m, 
                Err(e) => {
                    let st = stats.entry((s.sub, *label)).or_default();
                    st.n += 1; 
                    st.err += 1;
                    continue;
                }
            }; 
            //let img = pre.run(&s.img)?;
            let img = pre.run(&raw)?;
            let t0 = Instant::now(); 
            let res = decoder.detect(&img);
            let dt = t0.elapsed().as_secs_f64() * 1000.0; 

            let key = (s.sub, *label); 
            let st = stats.entry(key).or_default();
            st.n += 1;
            st.ms.push(dt); 

            let status = match &res {
                Err(_) => "ERR".to_string(), 
                Ok(r) if r.is_empty() => "MISS".to_string(), 
                Ok(r) if r.iter().any(|d| matches(&s.expected, &d.data)) => "OK".to_string(),
                Ok(_) => "WRONG".to_string()   
            }; 
            eprintln!(
                "[{label}] [{}] {}/{total} {} -> {status} ({dt:.1} ms)", 
                s.sub.label(), idx + 1, s.file
            );

            match res {
                Err(_) => st.err += 1, 
                Ok(r) => {
                    let mut image_correct = false;
                    for d in r.iter() {
                        let g = normalize(&d.data); 
                        if g.is_empty() {
                            continue;
                        }
                        st.total_returned += 1;
                        if matches(&s.expected, &d.data) {
                            st.correct_returns += 1;
                            image_correct = true;
                        }
                    }
                    if image_correct {
                        st.correct_barcodes += 1;
                    }
                }
            }
        }
    }

    println!(
        "\n{:<14} {:<18} {:>4} {:>12} {:>10} {:>7} {:>9} {:>9}",
        "sub-dataset", "combo", "n", "reading_rate", "precision", "err%", "p50(ms)", "p95(ms)"
    );
    for ((sub, label), st) in stats.iter_mut() {
        st.ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let errp = 100.0 * st.err as f64 / st.n as f64;
        println!(
            "{:<14} {:<18} {:>4} {:>10.2}% {:>9.2}% {:>6.1}% {:>9.2} {:>9.2}",
            sub.label(), label, st.n, st.reading_rate(), st.precision(), errp,
            percentile(&st.ms, 0.5), percentile(&st.ms, 0.95),
        );
    }

    Ok(())
}
