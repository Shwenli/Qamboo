import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os

def plot_execution_time(input_file, output_file):
    try:
        # Read the CSV file
        df = pd.read_csv(input_file)
        
        # Strip whitespace from column names just in case
        df.columns = df.columns.str.strip()
        
        # Identify columns (robustly handle potential naming variations)
        # Using the first column as Query and second as Time if names don't match exactly
        query_col = df.columns[0]      # Column 1: Query
        qamboo_time_col = df.columns[3] # Column 2: Qamboo
        orq_time_col = df.columns[4]    # Column 3: ORQ
        

        
        print(f"Using columns: Query='{query_col}', Time='{qamboo_time_col}', ORQ='{orq_time_col}'")

        # Set global font
        plt.rcParams['font.family'] = 'sans-serif'
        plt.rcParams['font.sans-serif'] = ['Arial']
        
        # Create 2 subplots (stacked vertically)
        fig,ax = plt.subplots(figsize=(10, 1.6))
        
        bar_width = 0.35       # Bar width
        
        # Plot the first subplot (first 11 queries)
        x1 = np.arange(22)
        ax.bar(x1 - bar_width/2, df[orq_time_col], width=bar_width, 
                color="#D5A3BF", edgecolor="#D5A3BF", linewidth=0.8, 
                label='ORQ', alpha=0.9)
        ax.bar(x1 + bar_width/2, df[qamboo_time_col], width=bar_width, 
                color="#929FBA", edgecolor="#929FBA", linewidth=0.8, 
                label='Qamboo', alpha=0.9)
        
        ax.set_ylabel('Row Bandwith (KB)', fontsize=9, fontweight='bold')
        #ax.set_yticks([0, 5, 10, 15])
        #ax.set_ylim(0, 15)
        ax.yaxis.grid(True, linestyle='--', linewidth=0.5, alpha=0.7, color='gray')
        ax.set_axisbelow(True)
        ax.legend(loc='upper left', frameon=True, fancybox=False, 
                   edgecolor='black', fontsize=8)
        
        ax.spines['top'].set_visible(False)
        ax.spines['right'].set_visible(False)
        ax.spines['left'].set_linewidth(1.2)
        ax.spines['bottom'].set_linewidth(1.2)

        ax.set_xticks(x1)
        ax.set_xticklabels(df[query_col], rotation=0, fontsize=8)
        ax.tick_params(axis='y', labelsize=8)
        ax.locator_params(axis='y', nbins=4)
        
        # Annotate Q6 values (index 5) since bars are small
        q6_idx = 5
        ax.text(q6_idx - bar_width/2, df[orq_time_col].iloc[q6_idx] + 3,
                f"{df[orq_time_col].iloc[q6_idx]:.1f}",
                ha='center', va='bottom', fontsize=8, color='#333333', rotation=90)
        ax.text(q6_idx + bar_width/2, df[qamboo_time_col].iloc[q6_idx] + 3,
                f"{df[qamboo_time_col].iloc[q6_idx]:.1f}",
                ha='center', va='bottom', fontsize=8, color='#333333', rotation=90)
        
        # Use fixed margins instead of tight_layout to guarantee exact horizontal alignment across figures
        fig.subplots_adjust(left=0.08, right=0.99, bottom=0.15, top=0.99)
        
        # Save - omit bbox_inches='tight' to avoid auto-cropping to different widths due to varying y-axis label content
        plt.savefig(output_file, format='pdf', dpi=300, facecolor='white', edgecolor='none')
        plt.savefig(os.path.splitext(output_file)[0] + '.png', format='png', dpi=300, facecolor='white', edgecolor='none')
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

    parser.add_argument('input_file', type=str, nargs='?', default=os.path.join(data_dir, 'fig8.csv'),
                        help='Path to the input CSV file (default: ../data/fig8.csv)')
    parser.add_argument('output_file', type=str, nargs='?', default=None,
                        help='Path to the output PDF file (optional, defaults to input filename + .pdf)')
    
    args = parser.parse_args()
    
    # Handle the input file path
    csv_path = args.input_file
    
    # If no output file is specified, use the input filename (without extension) + .pdf
    if args.output_file is None:
        base_name = os.path.splitext(os.path.basename(csv_path))[0]
        output_image_path = os.path.join(figures_dir, base_name + '.pdf')
    else:
        output_image_path = args.output_file
        # Automatically append the .pdf suffix (if not already present)
        if not output_image_path.endswith('.pdf'):
            output_image_path += '.pdf'
    
    plot_execution_time(csv_path, output_image_path)
