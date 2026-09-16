# Final release-build observations

Each pair is baseline / candidate. Times are milliseconds; peak RSS is MiB. All 24 cases remained below the provisional review triggers. Raw first observations, 15 warm pairs, counts and bytes are in the evidence JSON.

| Workload | Command | Query | Median ms | p95 ms | Output bytes | Peak RSS MiB |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| current-workspace | search | prefix | 12.21 / 12.25 | 12.96 / 13.58 | 3208 / 3208 | 7.1 / 7.0 |
| current-workspace | search | tail | 11.98 / 12.17 | 12.63 / 13.56 | 75 / 1137 | 7.0 / 7.0 |
| current-workspace | search | absent | 12.19 / 12.32 | 13.22 / 12.94 | 75 / 75 | 7.1 / 7.1 |
| current-workspace | search | common | 13.25 / 13.15 | 15.04 / 16.43 | 45665 / 45614 | 7.0 / 7.0 |
| current-workspace | context | prefix | 13.34 / 13.56 | 13.98 / 14.37 | 22185 / 22185 | 8.2 / 8.0 |
| current-workspace | context | tail | 13.01 / 12.72 | 14.99 / 14.70 | 13595 / 16473 | 8.1 / 8.1 |
| current-workspace | context | absent | 12.56 / 12.60 | 13.60 / 13.51 | 13609 / 13609 | 8.2 / 8.1 |
| current-workspace | context | common | 14.62 / 14.33 | 15.19 / 15.75 | 79934 / 80508 | 8.1 / 8.1 |
| many-short | search | prefix | 55.15 / 56.58 | 59.58 / 60.17 | 29230 / 29230 | 5.5 / 5.6 |
| many-short | search | tail | 53.89 / 54.29 | 58.72 / 59.33 | 29230 / 29230 | 5.5 / 5.7 |
| many-short | search | absent | 53.55 / 53.63 | 59.51 / 56.67 | 75 / 75 | 5.4 / 5.4 |
| many-short | search | common | 56.68 / 56.07 | 72.33 / 67.65 | 29230 / 29230 | 5.5 / 5.6 |
| many-short | context | prefix | 53.84 / 55.26 | 56.61 / 59.08 | 30299 / 30299 | 6.2 / 6.3 |
| many-short | context | tail | 55.08 / 54.45 | 58.45 / 60.38 | 30297 / 30297 | 6.2 / 6.3 |
| many-short | context | absent | 53.11 / 52.87 | 63.05 / 61.59 | 1147 / 1147 | 6.1 / 6.1 |
| many-short | context | common | 55.91 / 55.67 | 63.75 / 59.15 | 30299 / 30299 | 6.2 / 6.3 |
| near-budget | search | prefix | 78.04 / 77.10 | 87.52 / 86.28 | 44830 / 44830 | 68.6 / 68.7 |
| near-budget | search | tail | 73.07 / 83.96 | 79.24 / 92.38 | 75 / 44880 | 68.5 / 69.3 |
| near-budget | search | absent | 75.89 / 77.50 | 83.26 / 88.10 | 75 / 75 | 68.5 / 68.5 |
| near-budget | search | common | 72.56 / 71.66 | 78.27 / 80.86 | 44830 / 44830 | 68.6 / 68.8 |
| near-budget | context | prefix | 73.51 / 75.20 | 80.92 / 80.67 | 45901 / 45901 | 69.6 / 69.8 |
| near-budget | context | tail | 72.40 / 85.22 | 80.01 / 96.44 | 1147 / 45949 | 69.4 / 70.3 |
| near-budget | context | absent | 77.37 / 78.81 | 89.15 / 84.33 | 1149 / 1149 | 69.4 / 69.5 |
| near-budget | context | common | 71.82 / 70.15 | 82.86 / 79.87 | 45901 / 45901 | 69.7 / 69.6 |
