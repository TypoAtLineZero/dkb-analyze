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
![alt text](http://dkb-analyze-overview.png)