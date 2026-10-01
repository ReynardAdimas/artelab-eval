use barcode_scanner_rs::{
    detector::BarcodeDetector,
    preprocess::{PreprocessKind, Preprocessor},
};
use std::{collections::BTreeMap, path::PathBuf, time::Instant};

mod ground_truth;
mod localized_rxing;
mod localizer;
mod soros;

use ground_truth::{load_all, load_deal_kaist, load_muenster, SubDataset};
use localized_rxing::LocalizedRxingDetector;
use localizer::{Localizer, OpenCvLocalizer};
use soros::{SorosConfig, SorosLocalizer};

fn normalize(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

enum LocalizerKind {
    OpenCv,
    Soros(SorosConfig),
}

enum DecoderKind {
    LocalizedRxing {
        loc: LocalizerKind,
        local_otsu: bool,
    },
}

#[derive(Default)]
struct Stat {
    n: usize,
    total_returned: usize,
    correct_returns: usize,
    correct_barcodes: usize,
    err: usize,
    ms: Vec<f64>,
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
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
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

/// Mengembalikan tipe konkret (bukan Box<dyn>) agar crop_hits / fallback_hits bisa dibaca.
fn build_decoder(kind: &DecoderKind) -> Result<LocalizedRxingDetector, String> {
    match kind {
        DecoderKind::LocalizedRxing { loc, local_otsu } => {
            let localizer: Box<dyn Localizer> = match loc {
                LocalizerKind::OpenCv => Box::new(OpenCvLocalizer::new()?),
                LocalizerKind::Soros(cfg) => Box::new(SorosLocalizer::new(cfg.clone())),
            };
            LocalizedRxingDetector::new(localizer, 2.0, 0.15, *local_otsu)
        }
    }
}

/// Pemakaian:
///   cargo run --release -- <data_root> [dataset] [filter]
///   dataset : deal (default) | artelab | muenster
///   filter  : substring label combo, mis. "soros" untuk hanya menjalankan combo Sörös
fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let data_root = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("data"));
    let dataset = args.get(2).map(String::as_str).unwrap_or("deal");
    let filter: Option<&str> = args.get(3).map(String::as_str);

    let samples = match dataset {
        "artelab" => load_all(&data_root)?,
        "muenster" => load_muenster(&data_root)?,
        _ => load_deal_kaist(&data_root)?,
    };

    let all_combos: Vec<(&str, PreprocessKind, DecoderKind)> = vec![
        // baseline: localizer bawaan OpenCV
        (
            "raw+localize+rxing",
            PreprocessKind::Raw,
            DecoderKind::LocalizedRxing { loc: LocalizerKind::OpenCv, local_otsu: false },
        ),
        (
            "gray+localize+rxing",
            PreprocessKind::Gray,
            DecoderKind::LocalizedRxing { loc: LocalizerKind::OpenCv, local_otsu: false },
        ),
        (
            "gray+localize_otsu+rxing",
            PreprocessKind::Gray,
            DecoderKind::LocalizedRxing { loc: LocalizerKind::OpenCv, local_otsu: true },
        ),
        // Sörös & Flörkemeier (MUM'13)
        (
            "raw+soros+rxing",
            PreprocessKind::Raw,
            DecoderKind::LocalizedRxing {
                loc: LocalizerKind::Soros(SorosConfig::default()),
                local_otsu: false,
            },
        ),
        // ablasi: tanpa filter saturasi HSV
        (
            "raw+soros_nohsv+rxing",
            PreprocessKind::Raw,
            DecoderKind::LocalizedRxing {
                loc: LocalizerKind::Soros(SorosConfig { sat_max: 1.0, ..Default::default() }),
                local_otsu: false,
            },
        ),
        // ablasi: threshold lebih ketat
        (
            "raw+soros_thr05+rxing",
            PreprocessKind::Raw,
            DecoderKind::LocalizedRxing {
                loc: LocalizerKind::Soros(SorosConfig { thr_ratio: 0.5, ..Default::default() }),
                local_otsu: false,
            },
        ),
    ];

    let combos: Vec<_> = all_combos
        .into_iter()
        .filter(|(label, _, _)| filter.map_or(true, |f| label.contains(f)))
        .collect();
    if combos.is_empty() {
        return Err(format!("Tidak ada combo yang cocok dengan filter {filter:?}"));
    }

    let mut stats: BTreeMap<(SubDataset, &str), Stat> = BTreeMap::new();
    let mut hits: Vec<(&str, u64, u64)> = Vec::new();

    for (label, kind, decoder_kind) in &combos {
        let mut pre = Preprocessor::new(*kind)?;
        let mut decoder = build_decoder(decoder_kind)?;
        let total = samples.len();

        for (idx, s) in samples.iter().enumerate() {
            let raw = match s.load_image() {
                Ok(m) => m,
                Err(_e) => {
                    let st = stats.entry((s.sub, *label)).or_default();
                    st.n += 1;
                    st.err += 1;
                    continue;
                }
            };
            let img = pre.run(&raw)?;
            let t0 = Instant::now();
            let res = decoder.detect(&img);
            let dt = t0.elapsed().as_secs_f64() * 1000.0;

            let key = (s.sub, *label);
            let st = stats.entry(key).or_default();
            st.n += 1;
            st.ms.push(dt);

            let status = match &res {
                Err(e) => format!("ERR: {}", {e}),
                Ok(r) if r.is_empty() => "MISS".to_string(),
                Ok(r) if r.iter().any(|d| matches(&s.expected, &d.data)) => "OK".to_string(),
                Ok(_) => "WRONG".to_string(),
            };
            eprintln!(
                "[{label}] [{}] {}/{total} {} -> {status} ({dt:.1} ms)",
                s.sub.label(),
                idx + 1,
                s.file
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

        hits.push((*label, decoder.crop_hits, decoder.fallback_hits));
    }

    println!(
        "\n{:<14} {:<26} {:>4} {:>12} {:>10} {:>7} {:>9} {:>9}",
        "sub-dataset", "combo", "n", "reading_rate", "precision", "err%", "p50(ms)", "p95(ms)"
    );
    for ((sub, label), st) in stats.iter_mut() {
        st.ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let errp = 100.0 * st.err as f64 / st.n as f64;
        println!(
            "{:<14} {:<26} {:>4} {:>10.2}% {:>9.2}% {:>6.1}% {:>9.2} {:>9.2}",
            sub.label(),
            label,
            st.n,
            st.reading_rate(),
            st.precision(),
            errp,
            percentile(&st.ms, 0.5),
            percentile(&st.ms, 0.95),
        );
    }

    println!("\n{:<26} {:>10} {:>14}", "combo", "crop_hits", "fallback_hits");
    for (label, crop, fallback) in &hits {
        println!("{:<26} {:>10} {:>14}", label, crop, fallback);
    }

    Ok(())
}