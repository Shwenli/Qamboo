import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os

def plot_sort_scale(csv_file, output_file):
    # Read CSV data
    df = pd.read_csv(csv_file)
    
    # Separate 64-bit and 32-bit data
    df_64bit = df[df['Bit Type'] == '64bit'].copy()
    df_32bit = df[df['Bit Type'] == '32bit'].copy()
    
    # Compute the exponent for each row count (used for x-axis labels)
    def get_exponent(n):
        return int(np.log2(n))
    
    # Prepare data for each subplot
    environments = ['LAN', 'WAN']
    colors = {'LAN': "#ec9f5c", 'WAN': "#74a074"}
    markers = {'LAN': 'o', 'WAN': 's'}

    plt.rcParams['font.family'] = 'sans-serif'
    plt.rcParams['font.sans-serif'] = ['Arial']

    # Create the figure with two subplots sharing the y-axis
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(4, 1.8), sharey=True)
    
    # Get unique row counts and compute their exponents
    all_rows = sorted(df['Rows'].unique())
    x_positions = np.arange(len(all_rows))
    # Show one label per point using LaTeX formatting
    x_labels = [f'$2^{{{get_exponent(r)}}}$' for i, r in enumerate(all_rows)]
    
    def plot_subplot(ax, df_subset, title):
        # Get LAN No-RDMA data as the baseline
        lan_no_rdma_data = df_subset[df_subset['Environment'] == 'LAN'].sort_values('Rows')
        lan_no_rdma_times = dict(zip(lan_no_rdma_data['Rows'].values, lan_no_rdma_data['Time (s)'].values))
        
        for env in environments:
            env_data = df_subset[df_subset['Environment'] == env].sort_values('Rows')
            line, = ax.plot(x_positions, env_data['Time (s)'].values, 
                   label=env, 
                   color=colors[env],
                   marker=markers[env],
                   linewidth=2,
                   markersize=4)
            
            # Add annotations for WAN (value divided by LAN No-RDMA)
            # Start from the second point, adding one every other point
            if env in ['WAN']:
                for i, (idx, row) in enumerate(env_data.iterrows()):
                    # Start from the second point (i=1), add one every other point (i=1,3,5,...)
                    if i < 1 or i % 2 == 0 or i == len(env_data) - 1: 
                        continue
                    
                    x_pos = x_positions[list(all_rows).index(row['Rows'])]
                    y_val = row['Time (s)']
                    baseline = lan_no_rdma_times.get(row['Rows'], 1)
                    ratio = y_val / baseline if baseline != 0 else 0
                    
                    offset_y = 5
                    va_align = 'bottom'
                    
                    # Special case: last point of 64-bit WAN gets its annotation on the left, first point of 32-bit LAN RDMA on the right
                    ha_align = 'center'
                    offset_x = 0
                    
                    if env == 'WAN' and i == len(env_data) - 1:
                        # Last point of 64-bit WAN: annotation to the left of the point
                        ha_align = 'right'
                        offset_x = -7
                        offset_y = 0
                    
                    ax.annotate(f'{ratio:.2f}×',
                               xy=(x_pos, y_val),
                               xytext=(offset_x, offset_y),
                               textcoords='offset points',
                               fontweight='bold',
                               ha=ha_align,
                               va='center' if offset_x != 0 else va_align,
                               fontsize=5,
                               color='#333333')
        
        ax.set_title(title, fontsize=8, fontweight='bold')
        ax.set_xticks(x_positions)
        ax.set_xticklabels(x_labels)
        ax.tick_params(axis='y', labelsize=7, pad=2)  # Reduce spacing between tick labels and the axis
        ax.tick_params(axis='x', labelsize=7)
        # Set the legend font to Times New Roman
        legend = ax.legend(loc='upper left', edgecolor='black', frameon=True, fancybox=False, fontsize=6)
        
        #for text in legend.get_texts():
            #text.set_fontfamily('Times New Roman')
        ax.grid(True, linestyle='--', alpha=0.7)
        ax.set_yscale('log')
    
    # Plot the 64-bit subplot
    plot_subplot(ax1, df_64bit, '64-bit Keys')
    
    # Plot the 32-bit subplot
    plot_subplot(ax2, df_32bit, '32-bit Keys')
    
    # Shared x-axis and y-axis labels, centered
    fig.text(0.5, 0.05, 'Number of Rows', ha='center', fontsize=7, fontweight='bold')
    fig.text(0.02, 0.5, 'Time (s)', va='center', rotation='vertical', fontsize=7, fontweight='bold')
    
    plt.tight_layout(pad=1.5)
    plt.savefig(output_file, dpi=300, bbox_inches='tight', pad_inches=0.0)
    plt.savefig(os.path.splitext(output_file)[0] + '.png', dpi=300, bbox_inches='tight', pad_inches=0.0)
    print(f"Figure saved to: {output_file}")
    # plt.show()

if __name__ == "__main__":
    # Resolve paths relative to this script: ../data for inputs, ../figures for outputs
    script_dir = os.path.dirname(os.path.abspath(__file__))
    csv_path = os.path.join(script_dir, '..', 'data', 'fig14.csv')
    output_path = os.path.join(script_dir, '..', 'figures', 'fig14.pdf')
    
    # Ensure the output directory exists
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    
    plot_sort_scale(csv_path, output_path)
