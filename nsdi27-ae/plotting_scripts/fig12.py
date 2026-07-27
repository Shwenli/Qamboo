import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os

# Warm color palette (highlight the main scheme)
COLORS = {
    'primary': "#E9928D",       # Soft brick red - Qamboo bottom (Rdma)
    'primary_light': "#F0C2E9", # Light brick red - Qamboo top (difference)
    'secondary': '#F8E4BC',     # Pale beige - ORQ (de-emphasized)
    'accent': '#C75146',        # Deep red - accent
    'neutral': '#B8B8B8',       # Light gray
}

def plot_execution_time(input_file, output_file):
    try:
        # Read the CSV file
        df = pd.read_csv(input_file)
        
        # Strip whitespace from column names just in case
        df.columns = df.columns.str.strip()
        
        # Identify columns (robustly handle potential naming variations)
        # Using the first column as Query and second as Time if names don't match exactly
        query_col = df.columns[0]      # Column 1: Query
        tcp_time_col = df.columns[1] # Column 2: Qamboo
        rdma_time_col = df.columns[2]    # Column 3: ORQ
        
        print(f"Using columns: Query='{query_col}', TCP='{tcp_time_col}', RDMA='{rdma_time_col}'")

        # Set global font
        plt.rcParams['font.family'] = 'sans-serif'
        plt.rcParams['font.sans-serif'] = ['Arial']
        
        # Create figure
        fig, ax = plt.subplots(figsize=(10, 1.8), dpi=300)
        
        x = np.arange(len(df))  # x-axis position index
        bar_width = 0.35        # bar width
        
        
        # Two groups of bars side by side: ORQ | Qamboo (stacked)
        ax.bar(x - bar_width/2, df[tcp_time_col], width=bar_width, 
               color=COLORS['secondary'], edgecolor=COLORS['secondary'], linewidth=0.8, 
               label='TCP', alpha=0.9)
        
        # Qamboo bottom: Rdma (solid color)
        ax.bar(x + bar_width/2, df[rdma_time_col], width=bar_width, 
               color=COLORS['primary'], edgecolor=COLORS['primary'], linewidth=0.8, 
               label='RDMA', alpha=0.9)
    
        
        # Annotate Q6 values (index 5) since bars are relatively small
        q6_idx = 5
        ax.text(q6_idx - bar_width/2, df[tcp_time_col].iloc[q6_idx] + 20,
                f"{df[tcp_time_col].iloc[q6_idx]:.2f}",
                ha='center', va='bottom', fontsize=8, color='#333333', rotation=90)
        ax.text(q6_idx + bar_width/2, df[rdma_time_col].iloc[q6_idx] + 20,
                f"{df[rdma_time_col].iloc[q6_idx]:.2f}",
                ha='center', va='bottom', fontsize=8, color='#333333', rotation=90)
        
        # Style refinement
        #ax.set_xlabel('Query', fontsize=12, fontweight='bold')
        ax.set_ylabel('Time (s)', fontsize=10, fontweight='bold')
        #ax.set_title('Execution Time per Query', fontsize=14, fontweight='bold', pad=15)
        ax.set_xticks(x)
        ax.set_xticklabels(df[query_col], rotation=0, ha='center', fontsize=8, fontweight='500')
        ax.tick_params(axis='y', labelsize=8)
        # Set y-axis ticks to [250, 500]
        #ax.set_ylim(0, 500)
        #ax.set_yticks([0, 250, 500])
        
        # Grid lines
        ax.yaxis.grid(True, linestyle='--', linewidth=0.5, alpha=0.7, color='gray')
        ax.set_axisbelow(True)
        
        # Legend
        ax.legend(loc='upper left', frameon=True, fancybox=False, 
                  edgecolor='black', fontsize=8,)
        
        # Borders
        ax.spines['top'].set_visible(False)
        ax.spines['right'].set_visible(False)
        ax.spines['left'].set_linewidth(1.2)
        ax.spines['bottom'].set_linewidth(1.2)

        plt.tight_layout()
        
        # Save - high DPI ensures clarity
        plt.savefig(output_file, format='pdf', dpi=300, bbox_inches='tight', 
                    pad_inches=0, facecolor='white', edgecolor='none')
        plt.savefig(os.path.splitext(output_file)[0] + '.png', format='png', dpi=300, bbox_inches='tight',
                    pad_inches=0, facecolor='white', edgecolor='none')
        print(f"Plot saved to: {output_file}")
        
    except FileNotFoundError:
        print(f"Error: File not found at {input_file}")
    except Exception as e:
        print(f"An error occurred: {e}")

if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description='Plot execution time from CSV')
    # Resolve default paths relative to this script: ../data for inputs, ../figures for outputs
    script_dir = os.path.dirname(os.path.abspath(__file__))
    data_dir = os.path.join(script_dir, '..', 'data')
    figures_dir = os.path.join(script_dir, '..', 'figures')
    os.makedirs(figures_dir, exist_ok=True)

    parser.add_argument('input_file', type=str, nargs='?', default=os.path.join(data_dir, 'fig12.csv'),
                        help='Input CSV file path (default: ../data/fig12.csv)')
    parser.add_argument('output_file', type=str, nargs='?', default=None,
                        help='Output PDF file path (optional, defaults to input filename + .pdf)')
    
    args = parser.parse_args()
    
    # Handle input file path
    csv_path = args.input_file
    
    # If no output file is specified, use the input filename (without extension) + .pdf
    if args.output_file is None:
        base_name = os.path.splitext(os.path.basename(csv_path))[0]
        output_image_path = os.path.join(figures_dir, base_name + '.pdf')
    else:
        output_image_path = args.output_file
        # Automatically append .pdf suffix (if missing)
        if not output_image_path.endswith('.pdf'):
            output_image_path += '.pdf'
    
    plot_execution_time(csv_path, output_image_path)
