import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os

def main():
    # Path configuration
    script_dir = os.path.dirname(os.path.abspath(__file__))
    data_file = os.path.join(script_dir, '..', 'data', 'fig11.csv')
    pic_dir = os.path.join(script_dir, '..', 'figures')
    os.makedirs(pic_dir, exist_ok=True)

    plt.rcParams['font.family'] = 'sans-serif'
    plt.rcParams['font.sans-serif'] = ['Arial']

    # Read CSV file
    df = pd.read_csv(data_file)

    # Extract the needed columns (time columns are usually at indices 1, 3, 5; duplicate column names may get auto-added suffixes)
    col_query = df.columns[0]
    col_opt_time = df.columns[1]
    col_no_join_time = df.columns[3]
    col_no_sec_time = df.columns[5]

    # Create two subplots (top and bottom)
    def plot_grouped_bar_join_reorder(df_filtered, col_x, col_y1, col_y2, label1, label2, title, output_filename):
        fig, ax = plt.subplots(figsize=(5, 2))
        x = np.arange(len(df_filtered[col_x]))
        width = 0.35

        # First bar (Qamboo opt): solid color bar, on the left
        ax.bar(x + width/2, df_filtered[col_y1], width, label=label1, color="#AE8EB6", edgecolor="#AE8EB6")
        # Second bar (version without optimization): hatched bar, on the right
        ax.bar(x - width/2, df_filtered[col_y2], width, label=label2, color="white", edgecolor="#AE8EB6", hatch="///")

        ax.set_ylabel('Time (s)',fontweight='bold', fontsize=10)
        # ax.set_title(title) # For paper use, titles are usually omitted from figures; uncomment if needed
        ax.set_xticks(x)
        ax.set_xticklabels(df_filtered[col_x])
        ax.legend(frameon=True, fancybox=False, 
                   edgecolor='black', loc='upper left')
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
        print(f"Figure saved: {output_pdf}")


    def plot_grouped_bar_secure_cut(df_filtered, col_x, col_y1, col_y2, label1, label2, title, output_filename):
        fig, ax = plt.subplots(figsize=(5, 2))
        x = np.arange(len(df_filtered[col_x]))
        width = 0.35

        # First bar (Qamboo opt): solid color bar, on the left
        ax.bar(x + width/2, df_filtered[col_y1], width, label=label1, color="#D28783", edgecolor="#D28783")
        # Second bar (version without optimization): hatched bar, on the right
        ax.bar(x - width/2, df_filtered[col_y2], width, label=label2, color="white", edgecolor="#D28783", hatch="///")

        ax.set_ylabel('Time (s)',fontweight='bold', fontsize=10)
        # ax.set_title(title) # For paper use, titles are usually omitted from figures; uncomment if needed
        ax.set_xticks(x)
        ax.set_xticklabels(df_filtered[col_x])
        ax.legend(frameon=True, fancybox=False, 
                   edgecolor='black',)
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
        print(f"Figure saved: {output_pdf}")

    # Figure 1: opt vs no join reorder
    # Keep only rows that have no join reorder data
    df_join = df.dropna(subset=[col_no_join_time])
    plot_grouped_bar_join_reorder(df_join, col_query, col_opt_time, col_no_join_time, 
                     'Opt', 'No opt', 
                     'Qamboo opt vs Qamboo no join reorder', 
                     'fig11a.pdf')

    # Figure 2: opt vs no secure cut
    # Keep only rows that have no secure cut data
    df_sec = df.dropna(subset=[col_no_sec_time])
    plot_grouped_bar_secure_cut(df_sec, col_query, col_opt_time, col_no_sec_time, 
                     'Opt', 'No opt', 
                     'Qamboo opt vs Qamboo no secure cut', 
                     'fig11b.pdf')

if __name__ == "__main__":
    main()
