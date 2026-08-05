import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os

# Warm-toned academic color scheme (highlighting the main scheme)
COLORS = {
    'orq_lan': "#D5A3BF",          # Light beige - ORQ LAN (bottom)
    'qamboo_lan': "#929FBA",       # Soft brick red - Qamboo LAN (bottom)
}

def plot_execution_time(lan_file, wan_file, output_file):
    try:
        # Read both CSV files
        df_lan = pd.read_csv(lan_file)
        df_wan = pd.read_csv(wan_file)
        
        # Strip whitespace from column names
        df_lan.columns = df_lan.columns.str.strip()
        df_wan.columns = df_wan.columns.str.strip()
        
        # Identify columns
        query_col = df_lan.columns[0]
        lan_qamboo_col = df_lan.columns[1]  # Qamboo (column 2)
        lan_orq_col = df_lan.columns[2]      # ORQ (column 3)
        
        wan_qamboo_col = df_wan.columns[1]   # Qamboo
        wan_orq_col = df_wan.columns[2]      # Orq
        
        print(f"LAN columns: Query='{query_col}', Qamboo='{lan_qamboo_col}', ORQ='{lan_orq_col}'")
        print(f"WAN columns: Query='{query_col}', Qamboo='{wan_qamboo_col}', ORQ='{wan_orq_col}'")

        # Compute the extra time of WAN over LAN
        orq_wan_extra = df_wan[wan_orq_col] - df_lan[lan_orq_col]
        qamboo_wan_extra = df_wan[wan_qamboo_col] - df_lan[lan_qamboo_col]

        # Set global font
        plt.rcParams['font.family'] = 'sans-serif'
        plt.rcParams['font.sans-serif'] = ['Arial']
        plt.rcParams['hatch.linewidth'] = 2.0  # Set hatch linewidth
        
        # Create figure
        fig, ax = plt.subplots(figsize=(10, 1.6), dpi=300)
        
        x = np.arange(len(df_lan))  # x-axis position index
        bar_width = 0.35            # Bar width
        
        # Two groups of stacked bars side by side: ORQ (LAN + WAN extra) | Qamboo (LAN + WAN extra)
        
        # ORQ: LAN as the bottom, WAN extra time stacked on top
        ax.bar(x - bar_width/2, df_lan[lan_orq_col], width=bar_width, 
               color=COLORS['orq_lan'], edgecolor='none', 
               label='ORQ (LAN)', alpha=0.9)
        
        # Top hatched part: keep the background color, set the lines (edgecolor) to white, and set linewidth=0.5 for a subtle border if needed, or 0
        ax.bar(x - bar_width/2, orq_wan_extra, width=bar_width, 
               bottom=df_lan[lan_orq_col],
               color=COLORS['orq_lan'], edgecolor='white', linewidth=0, hatch="/////",
               label='ORQ (WAN)', alpha=0.9)
        
        # Qamboo: LAN as the bottom, WAN extra time stacked on top
        ax.bar(x + bar_width/2, df_lan[lan_qamboo_col], width=bar_width, 
               color=COLORS['qamboo_lan'], edgecolor='none', 
               label='Qamboo (LAN)', alpha=0.9)
        ax.bar(x + bar_width/2, qamboo_wan_extra, width=bar_width, 
               bottom=df_lan[lan_qamboo_col], hatch="/////",
               color=COLORS['qamboo_lan'], edgecolor='white', linewidth=0, 
               label='Qamboo (WAN)', alpha=0.9)
        
        ax.tick_params(axis='y', labelsize=8)
        ax.set_yticks([0, 800, 1600, 2400])  # Set y-axis ticks
        ax.set_ylabel('Time (s)', fontsize=10, fontweight='bold')
        ax.set_xticks(x)
        ax.set_xticklabels(df_lan[query_col], rotation=0, fontsize=8, fontweight='500')
        
        # Grid lines
        ax.yaxis.grid(True, linestyle='--', linewidth=0.5, alpha=0.7, color='gray')
        ax.set_axisbelow(True)
        
        # Legend - 2 rows, 2 columns
        ax.legend(loc='upper left', frameon=True, fancybox=False, 
                  edgecolor='black', fontsize=7, ncol=1)
        
        # Borders
        ax.spines['top'].set_visible(False)
        ax.spines['right'].set_visible(False)
        ax.spines['left'].set_linewidth(1.2)
        ax.spines['bottom'].set_linewidth(1.2)

        # Annotate Q6 values (index 5) since bars are too small to see
        q6_idx = 5
        orq_lan_val = df_lan[lan_orq_col].iloc[q6_idx]
        orq_total_val = df_wan[wan_orq_col].iloc[q6_idx]
        qamboo_lan_val = df_lan[lan_qamboo_col].iloc[q6_idx]
        qamboo_total_val = df_wan[wan_qamboo_col].iloc[q6_idx]
        
        # Place rotated labels directly above each Q6 bar
        ax.text(q6_idx - bar_width/2, 1200,
                f"{orq_lan_val:.2f} / {orq_total_val:.2f}",
                ha='center', va='top', fontsize=8, color='#333333', rotation=90)
        ax.text(q6_idx + bar_width/2, 1200,
                f"{qamboo_lan_val:.2f} / {qamboo_total_val:.2f}",
                ha='center', va='top', fontsize=8, color='#333333', rotation=90)
        
        # Use fixed margins instead of tight_layout to keep multiple figures perfectly aligned horizontally
        fig.subplots_adjust(left=0.08, right=0.99, bottom=0.15, top=0.99)
        
        # Save - drop bbox_inches='tight' to avoid auto-cropping to different widths due to different y-axis label contents
        plt.savefig(output_file, format='pdf', dpi=300, facecolor='white', edgecolor='none')
        plt.savefig(os.path.splitext(output_file)[0] + '.png', format='png', dpi=300, facecolor='white', edgecolor='none')
        print(f"Plot saved to: {output_file}")
        
    except FileNotFoundError as e:
        print(f"Error: File not found - {e}")
    except Exception as e:
        print(f"An error occurred: {e}")

if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description='Plot LAN vs WAN execution time comparison (stacked)')
    # Resolve default paths relative to this script: ../data/<source> for
    # inputs (paper = published data, run = freshly extracted results),
    # ../figures for outputs
    script_dir = os.path.dirname(os.path.abspath(__file__))

    parser.add_argument('--source', choices=['paper', 'run'], default='paper',
                        help='Data source directory: ../data/paper (published '
                             'numbers, default) or ../data/run (your own runs)')
    parser.add_argument('--lan', type=str, default=None,
                        help='Path to the LAN CSV file (default: ../data/<source>/fig7_lan_tpch.csv)')
    parser.add_argument('--wan', type=str, default=None,
                        help='Path to the WAN CSV file (default: ../data/<source>/fig7_wan_tpch.csv)')
    parser.add_argument('-o', '--output', type=str, default=None,
                        help='Path to the output PDF file (default: ../figures/<source>/fig7.pdf)')

    args = parser.parse_args()

    # Resolve input file paths
    data_dir = os.path.join(script_dir, '..', 'data', args.source)
    lan_path = args.lan or os.path.join(data_dir, 'fig7_lan_tpch.csv')
    wan_path = args.wan or os.path.join(data_dir, 'fig7_wan_tpch.csv')
    
    # Resolve output file path
    figures_dir = os.path.join(script_dir, '..', 'figures', args.source)
    os.makedirs(figures_dir, exist_ok=True)
    output_image_path = args.output or os.path.join(figures_dir, 'fig7.pdf')
    if not output_image_path.endswith('.pdf'):
        output_image_path += '.pdf'

    plot_execution_time(lan_path, wan_path, output_image_path)
