import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os

def plot_sort_scale(csv_file, output_file):
    # Read CSV data
    df = pd.read_csv(csv_file)
    
    # Separate 64-bit and 32-bit data
    df_64bit = df[df['bits'] == 64].copy()
    df_32bit = df[df['bits'] == 32].copy()
    
    # Prepare data for each subplot
    colors = {'Qamboo': "#ec9f5c", 'MP-SPDZ': "#74a074"}
    markers = {'Qamboo': 'o', 'MP-SPDZ': 's'}

    plt.rcParams['font.family'] = 'sans-serif'
    plt.rcParams['font.sans-serif'] = ['Arial']
    
    # Create figure with two subplots sharing the y-axis
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(4, 1.8), sharey=True)
    
    # Get unique rows values and compute the corresponding exponents
    all_rows = sorted(df['rows'].unique())
    x_positions = np.arange(len(all_rows))
    x_labels = [f"$2^{{{x}}}$" if (x - 16) % 2 == 0 else '' for x in range(16, 25)]
    
    def plot_subplot(ax, df_subset, title):
        # Plot Qamboo
        qamboo_data = df_subset.sort_values('rows')
        ax.plot(x_positions, qamboo_data['Qamboo_time'].values,
                label='Qamboo',
                color=colors['Qamboo'],
                marker=markers['Qamboo'],
                linewidth=2,
                markersize=4)
        
        # Plot MP-SPDZ
        mpspdz_data = df_subset.sort_values('rows')
        ax.plot(x_positions, mpspdz_data['mpspdz_time'].values,
                label='MP-SPDZ',
                color=colors['MP-SPDZ'],
                marker=markers['MP-SPDZ'],
                linewidth=2,
                markersize=3)
        
        # Add speedup annotations (MP-SPDZ time / Qamboo time)
        for i, (idx, row) in enumerate(qamboo_data.iterrows()):
            # Starting from the second point, annotate every other point
            if i < 1 or i % 2 == 0 or i == len(qamboo_data) - 1:
                continue
            
            x_pos = x_positions[i]
            qamboo_time = row['Qamboo_time']
            mpspdz_time = mpspdz_data.iloc[i]['mpspdz_time']
            speedup = mpspdz_time / qamboo_time if qamboo_time != 0 else 0
            
            # Annotate the speedup slightly above the Qamboo line
            ax.annotate(f'{speedup:.1f}×',
                       xy=(x_pos, qamboo_time),
                       xytext=(0, 5),
                       textcoords='offset points',
                       fontweight='bold',
                       ha='center',
                       va='bottom',
                       fontsize=5,
                       color='#333333')
        
        ax.set_title(title, fontsize=8, fontweight='bold')
        ax.set_xticks(x_positions)
        ax.set_xticklabels(x_labels)
        ax.tick_params(axis='y', labelsize=7, pad =2)
        ax.tick_params(axis='x', labelsize=7)
        
        # Set legend
        legend = ax.legend(loc='upper left', edgecolor='black', frameon=True, fancybox=False, fontsize=6)
        
        ax.grid(True, linestyle='--', alpha=0.7)
        ax.set_yscale('log')
    
    # Plot the 64-bit subplot
    plot_subplot(ax1, df_64bit, '64-bit Keys')
    
    # Plot the 32-bit subplot
    plot_subplot(ax2, df_32bit, '32-bit Keys')
    
    # Shared x and y axis labels, centered
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
    csv_path = os.path.join(script_dir, '..', 'data', 'fig10.csv')
    output_path = os.path.join(script_dir, '..', 'figures', 'fig10.pdf')
    
    # Ensure the output directory exists
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    
    plot_sort_scale(csv_path, output_path)
