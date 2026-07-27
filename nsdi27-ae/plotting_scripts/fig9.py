import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os

# Warm scientific color scheme (highlighting the main scheme)
COLORS = {
    'qamboo': "#929FBA",       # Soft blue - Qamboo
    'secrecy': "#E9928D",      # Deep red - Secrecy
}

def plot_execution_time(file, output_file):
    try:
        # Read both CSV files
        df = pd.read_csv(file)
        
        # Strip whitespace from column names
        df.columns = df.columns.str.strip()
        
        # Identify columns
        query_col = df.columns[0]
        qamboo_col = df.columns[1]  # Qamboo (2nd column)
        secrecy_col = df.columns[2]    # Secrecy (3rd column)
        
        print(f"columns: Query='{query_col}', Qamboo='{qamboo_col}', Secrecy='{secrecy_col}'")

        # Set global font
        plt.rcParams['font.family'] = 'sans-serif'
        plt.rcParams['font.sans-serif'] = ['Arial']
        
        # Create figure
        fig, ax = plt.subplots(figsize=(5, 2), dpi=300)
        
        x = np.arange(len(df))  # X-axis position index
        bar_width = 0.3            # Bar width
        
        # Three bars side by side: Qamboo | ORQ | Secrecy
        # Secrecy
        ax.bar(x-0.15, df[secrecy_col], width=bar_width, 
               color=COLORS['secrecy'], edgecolor='none',
               label='Secrecy', alpha=0.9)
        
        
        # Qamboo
        ax.bar(x+0.15, df[qamboo_col], width=bar_width, 
               color=COLORS['qamboo'], edgecolor='none', 
               label='Qamboo', alpha=0.9)
        
        
        ax.set_xticks(x)
        ax.tick_params(axis='y', labelsize=9)
        
        # Set logarithmic scale
        ax.set_yscale('log')
        
        # Style optimization
        ax.set_ylabel('Time (s)', fontsize=9, fontweight='bold')
        ax.set_xticklabels(df[query_col], rotation=0, fontsize=9)
        
        # Grid lines
        #ax.yaxis.grid(True, linestyle='--', linewidth=0.5, alpha=0.7, color='gray')
        ax.set_axisbelow(True)
        
        # Legend - 2 rows, 2 columns
        ax.legend(loc='upper left', frameon=True, fancybox=False, 
                  edgecolor='black', fontsize=8, ncol=1)
        
        # Border
        ax.spines['top'].set_linewidth(1.2)
        ax.spines['right'].set_linewidth(1.2)
        ax.spines['left'].set_linewidth(1.2)
        ax.spines['bottom'].set_linewidth(1.2)

        # Use fixed margins instead of tight_layout to keep multiple plots perfectly aligned horizontally
        plt.tight_layout()
        
        # Save - omit bbox_inches='tight' to avoid auto-cropping to different widths due to varying y-axis label content
        plt.savefig(output_file, format='pdf', dpi=300, bbox_inches = 'tight', pad_inches = 0.03)
        plt.savefig(os.path.splitext(output_file)[0] + '.png', format='png', dpi=300, bbox_inches = 'tight', pad_inches = 0.03)
        print(f"Plot saved to: {output_file}")
        
    except FileNotFoundError as e:
        print(f"Error: File not found - {e}")
    except Exception as e:
        print(f"An error occurred: {e}")

if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description='Plot LAN vs WAN execution time comparison (stacked)')
    # Resolve default paths relative to this script: ../data for inputs, ../figures for outputs
    script_dir = os.path.dirname(os.path.abspath(__file__))
    data_dir = os.path.join(script_dir, '..', 'data')
    figures_dir = os.path.join(script_dir, '..', 'figures')
    os.makedirs(figures_dir, exist_ok=True)

    parser.add_argument('-p', '--path', type=str, default=os.path.join(data_dir, 'fig9.csv'),
                        help='Path to the CSV file (default: ../data/fig9.csv)')
    parser.add_argument('-o', '--output', type=str, default=os.path.join(figures_dir, 'fig9.pdf'),
                        help='Path to the output PDF file (default: ../figures/fig9.pdf)')
    
    args = parser.parse_args()
    
    # Process input file path
    path = args.path
    
    # Process output file path
    output_image_path = args.output
    if not output_image_path.endswith('.pdf'):
        output_image_path += '.pdf'
    
    plot_execution_time(path, output_image_path)
