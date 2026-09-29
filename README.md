References Benchmark : https://www.dynamsoft.com/codepool/barcode-scanning-accuracy-benchmark-and-comparison.html 

Dataset : 	Artelab Medium Barcode 1D Collection 

**Combination Tested**
| Label | Preprocessing | Decoder |
|---|---|---|
| `raw+rxing` | none | rxing alone |
| `gray+rxing` | grayscale only | rxing alone |
| `gray_otsu+rxing` | grayscale + global Otsu threshold | rxing alone |
| `gray_otsu+hybrid` | grayscale + global Otsu threshold | fallback chain: StandardQr → WeChat → rxing |

**Result so far:**
| sub-dataset | combo | n | reading_rate | precision | err% | p50 (ms) | p95 (ms) |
|---|---|---:|---:|---:|---:|---:|---:|
| autofocus | gray+rxing | 215 | 86.05% | 89.37% | 0.0% | 1.17 | 2.52 |
| autofocus | gray_otsu+hybrid | 215 | 77.21% | 90.22% | 0.0% | 1.36 | 3.34 |
| autofocus | gray_otsu+rxing | 215 | 77.21% | 90.22% | 0.0% | 0.90 | 2.12 |
| autofocus | raw+rxing | 215 | 86.05% | 89.37% | 0.0% | 1.19 | 2.79 |
| no_autofocus | gray+rxing | 215 | 11.16% | 64.86% | 0.0% | 0.09 | 0.68 |
| no_autofocus | gray_otsu+hybrid | 215 | 9.77% | 70.00% | 0.0% | 0.19 | 0.85 |
| no_autofocus | gray_otsu+rxing | 215 | 9.77% | 70.00% | 0.0% | 0.09 | 0.62 |
| no_autofocus | raw+rxing | 215 | 11.16% | 64.86% | 0.0% | 0.12 | 0.83 | 

### Comparison against the published benchmark (same dataset)
 
| SDK | Autofocus reading rate | No-autofocus reading rate |
|---|---:|---:|
| Dynamsoft Barcode Reader | 100% | 81.86% |
| Scandit | 91.63% | 79.07% |
| **ZXing-CPP** | **82.36%** | **10.23%** |
| pyZbar | 89.77% | 13.95% |
| **`gray+rxing` (this project)** | **86.05%** | **11.16%** |
