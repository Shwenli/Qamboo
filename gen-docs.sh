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
           radix_sort_single radix_sort_multi \
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

# Add a "home" link (back to index.html) at the top of the sidebar on every
# page. rustdoc loads static.files/storage-*.js in the <head> of every page,
# so appending the snippet there covers all items without touching the
# generated HTML files. The relative path to the root comes from each page's
# data-root-path attribute.
echo "Adding home link to all pages..."
for f in docs/static.files/storage-*.js; do
cat >> "$f" << 'EOF'

// Injected by gen-docs.sh: "home" link at the top of the sidebar on every page.
document.addEventListener("DOMContentLoaded", function () {
    var crate = document.querySelector("nav.sidebar .sidebar-crate");
    if (!crate || document.querySelector(".sidebar-home")) return;
    var vars = document.querySelector("meta[name=rustdoc-vars]");
    var root = (vars && vars.getAttribute("data-root-path")) || "./";
    var home = document.createElement("div");
    home.className = "sidebar-crate sidebar-home";
    home.innerHTML = '<h2><a href="' + root + 'index.html">&#8962; home</a></h2>';
    crate.parentNode.insertBefore(home, crate);
});
EOF
done

# Create the index page from the project README: the README body is rendered
# by pandoc (GFM) and wrapped in the rustdoc shell, so the module sidebar
# stays available. Images referenced by the README are copied alongside.
echo "Creating index.html from README.md..."
cp Qamboo_overview.png docs/

cat > docs/index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta name="generator" content="rustdoc">
<meta name="description" content="Qamboo: An Efficient and Scalable MPC Framework for Relational Analytics">
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
</div>
</nav>
<div class="sidebar-resizer"></div>
<main>
<div class="width-limiter">
<section id="main-content" class="content">
EOF

# Render the project README (GitHub-flavored Markdown) into the page body.
pandoc README.md -f gfm -t html5 --wrap=none >> docs/index.html

cat >> docs/index.html << 'EOF'
</section>
</div>
</main>
</body>
</html>
EOF

echo "  Created: index.html from README.md"

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
