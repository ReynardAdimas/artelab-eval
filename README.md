References Benchmark : https://www.dynamsoft.com/codepool/barcode-scanning-accuracy-benchmark-and-comparison.html 


## Combination Tested
| Label | Preprocessing | Decoder |
|---|---|---|
| `gray+localize+rxing` | grayscale, decoder crops internally | localizing -> crop -> rxing |
| `gray+localize_otsu+rxing` | grayscale, decoder crops internally | localize -> crop -> Otsu on crop -> rxing |
| `raw+localize_otsu+rxing` | none, decoder crops internally | localizing -> crop -> rxing |

## Dataset : 	Artelab Medium Barcode 1D Collection 

**Result so far:**
| sub-dataset | combo | n | reading_rate | precision | err% | p50 (ms) | p95 (ms) |
|---|---|---:|---:|---:|---:|---:|---:|
| autofocus | gray+localize+rxing | 215 | 96.28% | 95.95% | 0.0% | 1402.61 | 3683.74 |
| autofocus | gray+localize_otsu+rxing | 215 | 96.74% | 96.46% | 0.0% | 1836.92 | 5470.56 |
| autofocus | raw+localize+rxing | 215 | 96.74% | 96.00% | 0.0% | 1440.72 | 3864.38 |
| no_autofocus | gray+localize+rxing | 215 | 39.53% | 87.63% | 0.0% | 213.97 | 540.86 |
| no_autofocus | gray+localize_otsu+rxing | 215 | 27.91% | 76.92% | 0.0% | 273.87 | 689.03 |
| no_autofocus | raw+localize+rxing | 215 | 40.00% | 86.87% | 0.0% | 203.65 | 492.11 | 

### Comparison against the published benchmark (same dataset)
 
| SDK | Autofocus reading rate | No-autofocus reading rate |
|---|---:|---:|
| Dynamsoft Barcode Reader | 100% | 81.86% |
| Scandit | 91.63% | 79.07% |
| **ZXing-CPP** | **82.36%** | **10.23%** |
| pyZbar | 89.77% | 13.95% |
| **raw+localize+rxing** | **96.74%** | **40.00%** | 


## Dataset : 	Muenster BarcodeDB  

**Result so far:**
| sub-dataset | combo | n | reading_rate | precision | err% | p50 (ms) | p95 (ms) |
|---|---|---:|---:|---:|---:|---:|---:|
| muenster | gray+localize+rxing | 215 | 88.53% | 94.40% | 0.0% | 213.30 | 539.27 |
| muenster | gray+localize_otsu+rxing | 215 | 87.96% | 94.16% | 0.0% | 216.0 | 554.90 |
| muenster | raw+localize+rxing | 215 | 88.34% | 94.77% | 0.0% | 220.71 | 517.58 |

### Comparison against the published benchmark (same dataset)
 
| SDK | Reading rate | Precision |
|---|---:|---:|
| Dynamsoft Barcode Reader | 96.96% | 100% |
| Scandit | 93.26% | 100% |
| **ZXing-CPP** | **75.14%** | **99.87%** |
| pyZbar | 70.59% | 95.63% |
| **gray+localize+rxing** | **88.53%** | **94.40%** | 

## Dataset : 	DEAL Lab Barcode Dataset 

**Result so far:**
| sub-dataset | combo | n | reading_rate | precision | err% | p50 (ms) | p95 (ms) |
|---|---|---:|---:|---:|---:|---:|---:|
| muenster | gray+localize+rxing | 215 | 75.55% | 91.40% | 0.0% | 186.32 | 2119.95 |
| muenster | gray+localize_otsu+rxing | 215 | 72.65% | 89.36% | 0.0% | 226.92 | 2383.90 |
| muenster | raw+localize+rxing | 215 | 75.45% | 91.39% | 0.0% | 182.52 | 2139.36 |

### Comparison against the published benchmark (same dataset)
 
| SDK | Reading rate | Precision |
|---|---:|---:|
| Dynamsoft Barcode Reader | 91.95% | 100% |
| Scandit | 61.05% | 100% |
| **ZXing-CPP** | **63.20%** | **97.98%** |
| pyZbar | 72.60% | 99.38% |
| **gray+localize+rxing** | **75.55%** | **91.40%** | 