#!/bin/bash
set -e

cd "$(dirname "$0")"

echo "Generating documentation..."

# Clean old docs
rm -rf target/doc docs

# Generate documentation for entire workspace
cargo doc --no-deps --workspace

# Copy to docs folder
cp -r target/doc docs/
touch docs/.nojekyll

# Remove binary documentation folders
echo "Cleaning up binary documentation..."
for bin in q{1..22} q{2,3,5,8,13,17,18,20,21}_no_secure_cut \
           q{2,5,7,8,9,10}_no_join_reorder q4_no_semi \
           multi_keys_join radix_sort radix_sort_scalability radix_sort_mpspdz \
           comorbidity rcdiff aspirin pwd credit; do
    if [ -d "docs/$bin" ]; then
        rm -rf "docs/$bin"
        echo "  Removed folder: $bin"
    fi
done

# Clean up crates.js - remove binary crates from the list
echo "Cleaning up crates.js..."
cat > docs/crates.js << 'EOF'
window.ALL_CRATES = ["algebra","communication","experiments","net","operator","primitives","protocols","random","table"];
//{"start":21,"fragment_lengths":[9,14,14,6,11,13,12,9,8]}
EOF

# Create custom index.html with full Qamboo overview
cat > docs/index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta name="generator" content="rustdoc">
<meta name="description" content="Qamboo: Scalable Secure Collaborative Analytics in Cloud">
<title>Qamboo - Rust</title>
<script>if(window.location.protocol!=="file:")document.head.insertAdjacentHTML("beforeend","SourceSerif4-Regular-6b053e98.ttf.woff2,FiraSans-Italic-81dc35de.woff2,FiraSans-Regular-0fe48ade.woff2,FiraSans-MediumItalic-ccf7e434.woff2,FiraSans-Medium-e1aa3f0a.woff2,SourceCodePro-Regular-8badfe75.ttf.woff2,SourceCodePro-Semibold-aa29a496.ttf.woff2".split(",").map(f=>`<link rel="preload" as="font" type="font/woff2" crossorigin href="static.files/${f}">`).join(""))</script>
<link rel="stylesheet" href="static.files/normalize-9960930a.css">
<link rel="stylesheet" href="static.files/rustdoc-aa0817cf.css">
<meta name="rustdoc-vars" data-root-path="./" data-static-root-path="./static.files/" data-current-crate="qamboo" data-themes="" data-resource-suffix="" data-rustdoc-version="1.90.0" data-channel="1.90.0" data-search-js="static.files/search-fa3e91e5.js" data-settings-js="static.files/settings-5514c975.js">
<script src="static.files/storage-68b7e25d.js"></script>
<script defer src="crates.js"></script>
<script defer src="static.files/main-eebb9057.js"></script>
<noscript><link rel="stylesheet" href="static.files/noscript-32bb7600.css"></noscript>
<link rel="alternate icon" type="image/png" href="static.files/favicon-32x32-6580c154.png">
<link rel="icon" type="image/svg+xml" href="static.files/favicon-044be391.svg">
</head>
<body class="rustdoc mod crate">
<!--[if lte IE 11]><div class="warning">This old browser is unsupported and will most likely display funky things.</div><![endif]-->
<nav class="mobile-topbar"><button class="sidebar-menu-toggle" title="show sidebar"></button></nav>
<nav class="sidebar">
<div class="sidebar-crate">
<h2><a href="index.html">Qamboo</a></h2>
</div>
<div class="sidebar-elems">
<section>
<h2><a href="#modules">Modules</a></h2>
<ul class="block mod">
<li><a href="algebra/index.html">algebra</a></li>
<li><a href="communication/index.html">communication</a></li>
<li><a href="net/index.html">net</a></li>
<li><a href="operator/index.html">operator</a></li>
<li><a href="primitives/index.html">primitives</a></li>
<li><a href="protocols/index.html">protocols</a></li>
<li><a href="random/index.html">random</a></li>
<li><a href="table/index.html">table</a></li>
<li><a href="experiments/index.html">experiments</a></li>
</ul>
</section>
<section>
<h2><a href="#architecture">Architecture</a></h2>
</section>
<section>
<h2><a href="#benchmarks">Benchmarks</a></h2>
</section>
</div>
</nav>
<div class="sidebar-resizer"></div>
<main>
<div class="width-limiter">
<div class="sub-container">
<h1 class="fqn"><span class="in-band">Qamboo Workspace</span></h1>
</div>
<section id="main-content" class="content">

<p>Qamboo is a high-performance Secure Multi-Party Computation (MPC) framework designed for <strong>scalable secure collaborative analytics</strong> in cloud environments.</p>

<p>By leveraging replicated secret sharing and novel low-overhead cryptographic protocols, Qamboo enables multiple distrusting parties to jointly analyze their combined datasets without revealing sensitive information. The implementation supports the full TPC-H benchmark suite with significantly reduced communication overhead compared to prior approaches.</p>

<h2 id="architecture"><a class="doc-anchor" href="#architecture">§</a>Architecture</h2>

<pre class="text rust-example-rendered"><code>Application Code
       ↓
   table (SQL-like interface)
       ↓
   operator (relational operators)
       ↓
   primitives (crypto primitives)
       ↓
   protocols (MPC protocols)
       ↓
   communication + net (network layer)</code></pre>

<h2 id="modules"><a class="doc-anchor" href="#modules">§</a>Workspace Crates</h2>

<table><thead><tr><th>Crate</th><th>Description</th></tr></thead><tbody>
<tr><td><a href="algebra/index.html"><code>algebra</code></a></td><td>Foundational algebraic structures for MPC. Ring theory abstractions over <code>Z_{2^k}</code> with zero-copy serialization.</td></tr>
<tr><td><a href="communication/index.html"><code>communication</code></a></td><td>Optimized communication primitives for 3-party REP3. Zero-copy serialization and multi-threaded network operations.</td></tr>
<tr><td><a href="net/index.html"><code>net</code></a></td><td>High-performance network abstractions for MPC. Fast TCP with connection pooling, vectored I/O, and userspace buffering.</td></tr>
<tr><td><a href="operator/index.html"><code>operator</code></a></td><td>Privacy-preserving relational database operators: Join, GroupBy, Sort, Distinct, and Aggregation.</td></tr>
<tr><td><a href="primitives/index.html"><code>primitives</code></a></td><td>Cryptographic primitives built on MPC protocols: comparison, permutation, shuffling, MUX, division, and transforms.</td></tr>
<tr><td><a href="protocols/index.html"><code>protocols</code></a></td><td>Core MPC protocols. Semi-honest 3-party replicated secret sharing (REP3) over rings <code>Z_{2^k}</code>.</td></tr>
<tr><td><a href="random/index.html"><code>random</code></a></td><td>Secure correlated randomness generation. PRF-based expansion that eliminates online communication for random values.</td></tr>
<tr><td><a href="table/index.html"><code>table</code></a></td><td>High-level secure table operations. Columnar secure database with SQL-like query interface.</td></tr>
<tr><td><a href="experiments/index.html"><code>experiments</code></a></td><td>Benchmark suite and evaluation: TPC-H Q1–Q22, operator micro-benchmarks, and privacy-preserving applications.</td></tr>
</tbody></table>

<h2 id="benchmarks"><a class="doc-anchor" href="#benchmarks">§</a>Benchmarks</h2>

<p>The <a href="experiments/index.html"><code>experiments</code></a> crate provides comprehensive benchmarks, exposed as standalone binaries:</p>
<ul>
<li><strong>TPC-H</strong>: <code>q1</code> – <code>q22</code></li>
<li><strong>Operator Micro-benchmarks</strong>: <code>multi_keys_join</code>, <code>radix_sort</code>, <code>radix_sort_scalability</code>, <code>radix_sort_mpspdz</code></li>
<li><strong>Optimization Ablations</strong>: <code>q*_no_secure_cut</code>, <code>q*_no_join_reorder</code>, <code>q4_no_semi</code></li>
<li><strong>Privacy-Preserving Apps</strong>: <code>comorbidity</code>, <code>aspirin</code>, <code>credit</code>, <code>pwd</code>, <code>rcdiff</code></li>
</ul>

<p>Each binary has its own rustdoc page (e.g. <code>q1</code>, <code>multi_keys_join</code>), but they are all indexed from the <a href="experiments/index.html#binary-targets"><code>experiments</code></a> crate documentation. See the <code>scripts/</code> directory for automation scripts.</p>

<h2 id="security-model"><a class="doc-anchor" href="#security-model">§</a>Security Model</h2>

<p>Qamboo implements protocols secure in the <strong>semi-honest model</strong> with <strong>3 parties</strong>:</p>
<ul>
<li><strong>Privacy</strong>: No single party learns others' private inputs</li>
<li><strong>Robustness</strong>: Tolerates collusion of up to 1 party</li>
<li><strong>Cloud-Ready</strong>: Designed for honest-majority scenarios in cloud environments</li>
</ul>

<h2 id="license"><a class="doc-anchor" href="#license">§</a>License</h2>

<p>Licensed under either of <a href="https://opensource.org/licenses/MIT">MIT</a> or <a href="https://opensource.org/licenses/Apache-2.0">Apache-2.0</a> at your option.</p>

</section>
</div>
</main>
</body>
</html>
EOF

echo "  Created: custom index.html"

# Calculate size
SIZE=$(du -sh docs/ | cut -f1)
FILES=$(find docs/ -type f | wc -l)

echo ""
echo "✓ Documentation generated successfully!"
echo "  Size: $SIZE"
echo "  Files: $FILES"
echo "  Crates: 9 library crates only"
echo ""
echo "To preview locally:"
echo "  cd docs && python3 -m http.server 8080"
echo ""
echo "To deploy to GitHub Pages:"
echo "  git add docs/"
echo "  git commit -m 'Update documentation'"
echo "  git push"
