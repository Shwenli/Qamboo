import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os

def main():
    import argparse

    parser = argparse.ArgumentParser(description='Plot TPC-H scaling from CSV')
    parser.add_argument('--source', choices=['paper', 'run'], default='paper',
                        help='Data source directory: ../data/paper (published '
                             'numbers, default) or ../data/run (your own runs)')
    args = parser.parse_args()

    # Path configuration
    script_dir = os.path.dirname(os.path.abspath(__file__))
    data_file = os.path.join(script_dir, '..', 'data', args.source, 'fig13.csv')
    pic_dir = os.path.join(script_dir, '..', 'figures', args.source)
    os.makedirs(pic_dir, exist_ok=True)

    plt.rcParams['font.family'] = 'sans-serif'
    plt.rcParams['font.sans-serif'] = ['Arial']

    # Read CSV file
    df = pd.read_csv(data_file)

    # Extract required columns (time columns are usually at indices 1, 3, 5; duplicate column names may get auto-appended suffixes)
    col_query = df.columns[0]
    col_ratio = df.columns[3]

    def plot_ratio_bar(df_filtered, col_x, col_y, output_filename):
        fig, ax = plt.subplots(figsize=(5, 1.8))
        x = np.arange(len(df_filtered[col_x]))
        width = 0.55

        ax.bar(x, df_filtered[col_y], width, color="#929FBA", edgecolor="#929FBA")

        ax.set_ylabel('Ratio',fontweight='bold', fontsize=8)
        # Set y-axis ticks: 5, 10, 15, plus one tick at the maximum value
        max_val = df_filtered[col_y].max()
        y_max = int(np.ceil(max_val))
        y_ticks = [5, 10, y_max]
        ax.set_ylim(0, y_max+1)
        ax.set_yticks(y_ticks)
        # ax.set_title(title) # Titles are usually omitted in paper figures; uncomment if needed
        ax.set_xticks(x)
        ax.set_xticklabels(df_filtered[col_x], rotation=90, fontsize=8)
        ax.grid(axis='y', linestyle='--', alpha=0.7)

        ax.spines['top'].set_linewidth(1.2)
        ax.spines['right'].set_linewidth(1.2)
        ax.spines['left'].set_linewidth(1.2)
        ax.spines['bottom'].set_linewidth(1.2)
        
        plt.tight_layout()
        output_pdf = os.path.join(pic_dir, output_filename)
        plt.savefig(output_pdf, dpi=300, bbox_inches = 'tight', pad_inches = 0.03)
        plt.savefig(os.path.splitext(output_pdf)[0] + '.png', dpi=300, bbox_inches = 'tight', pad_inches = 0.03)
        plt.close(fig)
        print(f"Figure generated: {output_pdf}")

    plot_ratio_bar(df, col_query, col_ratio, 'fig13.pdf')

if __name__ == "__main__":
    main()
