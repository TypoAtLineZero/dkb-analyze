# DKB Analyze

Library to analyze bank account exports from German DKB.

## Implementation

1. Parse flags
    1. Input Files(s)
    2. Analysis [default: monthly, weekly, yearly]
    3. Visualization [default: true, false]
2. Perform analysis
    1. Reading file(s)
    2. Grooming (Headers, empty lines)
    3. Cluster bookings (User input needed? Maybe as dialogue? Template file?)
    4. Visualization [default: Sankey, Pie chart?]

## Usage

Input:

- Single, raw export
- Mutiple, raw exports

Output:

- Weekly, monthly, yearly stacked bar chart with accumulated account bookings

Abstraction overview
![Model](https://github.com/TypoAtLineZero/dkb-analyze/blob/main/dkb-analyze-overview.png)

<p align="center" width="100%">
<a href="https://buymeacoffee.com/typoatlinezero" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: 41px !important;width: 174px !important;box-shadow: 0px 3px 2px 0px rgba(190, 190, 190, 0.5) !important;-webkit-box-shadow: 0px 3px 2px 0px rgba(190, 190, 190, 0.5) !important;" ></a>
</p>
