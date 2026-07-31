//! # Graphify Export — JSON, HTML, Markdown Exports
//!
//! Exports the knowledge graph to various formats:
//! - JSON (graph.json)
//! - HTML interactive visualization (graph.html) using D3.js
//! - Markdown architectural report (GRAPH_REPORT.md)

use graphify_core::KnowledgeGraph;
use graphify_core::community::Community;
use graphify_core::metrics::NodeMetrics;
#[cfg(test)]
use graphify_core::GraphStats;
use std::path::Path;
use std::collections::HashMap;

/// Export graph to JSON.
pub fn export_json(kg: &KnowledgeGraph) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(kg)
}

/// Export graph to an interactive HTML visualization.
pub fn export_html(kg: &KnowledgeGraph, communities: &[Community]) -> String {
    let graph_data = serde_json::to_string(kg).unwrap_or_else(|_| "{}".into());
    let communities_data = serde_json::to_string(communities).unwrap_or_else(|_| "[]".into());

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Graphify Pro — Knowledge Graph</title>
<style>
* {{ margin: 0; padding: 0; box-sizing: border-box; }}
body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif; background: #0d1117; color: #c9d1d9; overflow: hidden; }}
#app {{ display: flex; height: 100vh; width: 100vw; }}
#sidebar {{ width: 340px; background: #161b22; border-right: 1px solid #30363d; overflow-y: auto; padding: 20px; display: flex; flex-direction: column; gap: 16px; flex-shrink: 0; }}
#canvas {{ flex: 1; position: relative; }}
svg {{ width: 100%; height: 100%; }}
h1 {{ font-size: 1.25rem; font-weight: 700; color: #f0f6fc; }}
h2 {{ font-size: 0.9rem; font-weight: 600; color: #8b949e; text-transform: uppercase; letter-spacing: 0.05em; }}
.stat-grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }}
.stat {{ background: #0d1117; border: 1px solid #21262d; border-radius: 8px; padding: 10px; }}
.stat-label {{ font-size: 0.7rem; color: #8b949e; text-transform: uppercase; }}
.stat-value {{ font-size: 1.2rem; font-weight: 700; color: #58a6ff; }}
#node-list {{ max-height: 300px; overflow-y: auto; }}
.node-item {{ padding: 8px 10px; border-radius: 6px; cursor: pointer; font-size: 0.85rem; transition: background 0.15s; border-left: 3px solid transparent; }}
.node-item:hover {{ background: #21262d; }}
.node-item.god {{ border-left-color: #f78166; }}
.node-item.class {{ border-left-color: #58a6ff; }}
.node-item.function {{ border-left-color: #7ee787; }}
.node-item.file {{ border-left-color: #8b949e; }}
#tooltip {{ position: absolute; background: #21262d; border: 1px solid #30363d; border-radius: 8px; padding: 12px; font-size: 0.8rem; pointer-events: none; display: none; max-width: 280px; z-index: 100; }}
.community-legend {{ display: flex; flex-wrap: wrap; gap: 6px; }}
.legend-item {{ display: flex; align-items: center; gap: 6px; font-size: 0.75rem; }}
.legend-color {{ width: 12px; height: 12px; border-radius: 3px; flex-shrink: 0; }}
.search-box {{ width: 100%; padding: 8px 12px; background: #0d1117; border: 1px solid #30363d; border-radius: 6px; color: #c9d1d9; font-size: 0.85rem; outline: none; transition: border-color 0.2s; }}
.search-box:focus {{ border-color: #58a6ff; }}
</style>
</head>
<body>
<div id="app">
  <div id="sidebar">
    <h1>🔬 Graphify Pro</h1>
    <div class="stat-grid">
      <div class="stat"><div class="stat-label">Nodes</div><div class="stat-value">{node_count}</div></div>
      <div class="stat"><div class="stat-label">Edges</div><div class="stat-value">{edge_count}</div></div>
      <div class="stat"><div class="stat-label">Communities</div><div class="stat-value">{community_count}</div></div>
      <div class="stat"><div class="stat-label">Density</div><div class="stat-value">{density:.3}</div></div>
    </div>
    <div>
      <h2>Search</h2>
      <input class="search-box" id="search" placeholder="Find nodes..." oninput="filterNodes()" />
    </div>
    <div>
      <h2>Top Nodes</h2>
      <div id="node-list"></div>
    </div>
    <div>
      <h2>Communities</h2>
      <div class="community-legend" id="community-legend"></div>
    </div>
  </div>
  <div id="canvas">
    <svg id="graph-svg"></svg>
    <div id="tooltip"></div>
  </div>
</div>
<script src="https://d3js.org/d3.v7.min.js"></script>
<script>
const graphData = {graph_json};
const communities = {communities_json};

// Community colors
const COLORS = ['#58a6ff','#7ee787','#f78166','#ff7b72','#d2a8ff','#ffa657','#a5d6ff','#79c0ff','#56d364','#f0883e','#f778ba','#e3b341','#8b949e','#6e7681','#db6d28'];

// Build node map
const nodeMap = {{}};
graphData.nodes.forEach(n => nodeMap[n.id] = n);

// Build community map
const commMap = {{}};
communities.forEach(c => c.nodes.forEach(nid => commMap[nid] = c));

// Compute degrees
const degrees = {{}};
graphData.edges.forEach(e => {{ degrees[e.source] = (degrees[e.source]||0)+1; degrees[e.target] = (degrees[e.target]||0)+1; }});

// Create D3 force layout
const width = document.getElementById('canvas').clientWidth;
const height = document.getElementById('canvas').clientHeight;
const svg = d3.select('#graph-svg');
const g = svg.append('g');

const zoom = d3.zoom().scaleExtent([0.1, 8]).on('zoom', e => g.attr('transform', e.transform));
svg.call(zoom);

const links = graphData.edges.map(e => ({{ source: e.source, target: e.target, relation: e.relation, confidence: e.confidence }}));
const nodes = graphData.nodes.map(n => ({{ id: n.id, label: n.label, type: n.type, isGodNode: n.is_god_node, degree: degrees[n.id]||0 }}));

const simulation = d3.forceSimulation(nodes)
  .force('link', d3.forceLink(links).id(d => d.id).distance(d => d.confidence === 'EXTRACTED' ? 80 : 150))
  .force('charge', d3.forceManyBody().strength(d => d.isGodNode ? -500 : (d.degree > 10 ? -200 : -80)))
  .force('center', d3.forceCenter(width/2, height/2))
  .force('collision', d3.forceCollide().radius(d => d.isGodNode ? 25 : (d.degree > 10 ? 15 : 8)));

const link = g.append('g').selectAll('line').data(links).join('line')
  .attr('stroke', d => d.confidence === 'EXTRACTED' ? '#30363d' : d.confidence === 'INFERRED' ? '#3d4a30' : '#4a3030')
  .attr('stroke-width', d => d.confidence === 'EXTRACTED' ? 1 : 0.6)
  .attr('stroke-opacity', 0.5);

const node = g.append('g').selectAll('g').data(nodes).join('g')
  .attr('class', d => 'node ' + d.type)
  .call(d3.drag().on('start', dragstarted).on('drag', dragged).on('end', dragended));

node.append('circle')
  .attr('r', d => d.isGodNode ? 14 : Math.max(4, Math.min(10, Math.sqrt(d.degree+1)*2)))
  .attr('fill', d => {{
    const c = commMap[d.id]; return c ? COLORS[c.id % COLORS.length] : '#8b949e';
  }})
  .attr('stroke', d => d.isGodNode ? '#f78166' : '#30363d')
  .attr('stroke-width', d => d.isGodNode ? 2.5 : 1)
  .attr('opacity', 0.85);

node.append('text')
  .text(d => d.label.substring(0, 25))
  .attr('x', 16).attr('y', 4)
  .attr('font-size', d => d.isGodNode ? '10px' : '8px')
  .attr('fill', d => d.isGodNode ? '#f78166' : '#8b949e')
  .attr('font-weight', d => d.isGodNode ? '700' : '400');

node.on('mouseover', showTooltip).on('mouseout', hideTooltip);

simulation.on('tick', () => {{
  link.attr('x1', d => d.source.x).attr('y1', d => d.source.y)
      .attr('x2', d => d.target.x).attr('y2', d => d.target.y);
  node.attr('transform', d => `translate(${{d.x}},${{d.y}})`);
}});

function dragstarted(event, d) {{ if (!event.active) simulation.alphaTarget(0.3).restart(); d.fx = d.x; d.fy = d.y; }}
function dragged(event, d) {{ d.fx = event.x; d.fy = event.y; }}
function dragended(event, d) {{ if (!event.active) simulation.alphaTarget(0); d.fx = null; d.fy = null; }}

function showTooltip(event, d) {{
  const tip = document.getElementById('tooltip');
  const n = nodeMap[d.id];
  const c = commMap[d.id];
  tip.innerHTML = `<strong>${{n.label}}</strong><br/><span style="color:#8b949e">${{n.type}}</span><br/>Degree: ${{d.degree}}<br/>${{c ? 'Community: ' + c.label : ''}}`;
  tip.style.display = 'block';
  tip.style.left = (event.pageX + 15) + 'px';
  tip.style.top = (event.pageY - 15) + 'px';
}}
function hideTooltip() {{ document.getElementById('tooltip').style.display = 'none'; }}

// Build node list in sidebar
const nodeList = document.getElementById('node-list');
const sortedNodes = [...nodes].sort((a,b) => b.degree - a.degree).slice(0, 50);
sortedNodes.forEach(n => {{
  const div = document.createElement('div');
  div.className = `node-item ${{n.isGodNode?'god':''}} ${{n.type}}`;
  div.textContent = n.label;
  div.onclick = () => {{
    simulation.alphaTarget(0.3).restart();
    // Fly to node
    const transform = d3.zoomIdentity.translate(width/2 - n.x, height/2 - n.y).scale(2);
    svg.transition().duration(750).call(zoom.transform, transform);
  }};
  nodeList.appendChild(div);
}});

// Community legend
const legend = document.getElementById('community-legend');
communities.slice(0, 15).forEach(c => {{
  const item = document.createElement('div');
  item.className = 'legend-item';
  item.innerHTML = `<div class="legend-color" style="background:${{COLORS[c.id % COLORS.length]}}"></div><span>${{c.label}}</span>`;
  legend.appendChild(item);
}});

function filterNodes() {{
  const q = document.getElementById('search').value.toLowerCase();
  node.attr('opacity', d => q === '' || d.label.toLowerCase().includes(q) ? 1 : 0.1);
}}
</script>
</body>
</html>"#,
        graph_json = graph_data,
        communities_json = communities_data,
        node_count = kg.stats.node_count,
        edge_count = kg.stats.edge_count,
        community_count = communities.len(),
        density = kg.stats.density,
    )
}

/// Export graph to a Markdown architectural report.
pub fn export_markdown(
    kg: &KnowledgeGraph,
    communities: &[Community],
    god_nodes: &[(String, String, usize)],
    _metrics: &[NodeMetrics],
) -> String {
    let mut report = String::new();

    report.push_str(&format!(
        "# 🔬 Graphify Pro — Architecture Report\n\n\
         **Generated:** {}\n\
         **Project:** {}\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        kg.project_root,
    ));

    report.push_str("## 📊 Overview\n\n");
    report.push_str(&format!(
        "| Metric | Value |\n|--------|-------|\n\
         | Total Nodes | {} |\n\
         | Total Edges | {} |\n\
         | Communities | {} |\n\
         | Graph Density | {:.4} |\n\
         | Average Degree | {:.2} |\n\
         | Connected Components | {} |\n\
         | Is Connected | {} |\n",
        kg.stats.node_count,
        kg.stats.edge_count,
        communities.len(),
        kg.stats.density,
        kg.stats.avg_degree,
        kg.stats.connected_components,
        if kg.stats.is_connected { "✅ Yes" } else { "❌ No" },
    ));

    // Primary language
    if let Some(ref lang) = kg.metadata.primary_language {
        report.push_str(&format!("| Primary Language | {} |\n", lang));
    }

    // All languages
    if !kg.metadata.languages.is_empty() {
        report.push_str(&format!(
            "| Languages | {} |\n",
            kg.metadata.languages.join(", ")
        ));
    }

    report.push_str("\n---\n\n");

    // God nodes section
    if !god_nodes.is_empty() {
        report.push_str("## 🏛️ God Nodes (Architectural Hubs)\n\n");
        report.push_str("These are the most connected nodes in the codebase:\n\n");
        report.push_str("| Node | Connections |\n|------|-------------|\n");
        for (_, label, degree) in god_nodes {
            report.push_str(&format!("| {} | {} |\n", label, degree));
        }
        report.push_str("\n---\n\n");
    }

    // Communities section
    if !communities.is_empty() {
        report.push_str("## 🏘️ Communities (Subsystems)\n\n");
        report.push_str(&format!("Detected {} communities:\n\n", communities.len()));

        let mut sorted_communities: Vec<_> = communities.iter().collect();
        sorted_communities.sort_by_key(|c| std::cmp::Reverse(c.size));

        for comm in sorted_communities.iter().take(15) {
            report.push_str(&format!("### {} ({} nodes)\n\n", comm.label, comm.size));
            if !comm.hubs.is_empty() {
                report.push_str(&format!("**Hubs:** {}\n\n", comm.hubs.join(", ")));
            }
            if let Some(ref desc) = comm.description {
                report.push_str(&format!(">{}\n\n", desc));
            }
        }
        report.push_str("\n---\n\n");
    }

    // Edge confidence distribution
    report.push_str("## 🎯 Confidence Distribution\n\n");
    let cd = &kg.stats.confidence_distribution;
    report.push_str(&format!(
        "| Level | Count | Percentage |\n|-------|-------|------------|\n\
         | EXTRACTED | {} | {:.1}% |\n\
         | INFERRED | {} | {:.1}% |\n\
         | AMBIGUOUS | {} | {:.1}% |\n",
        cd.extracted,
        if kg.stats.edge_count > 0 { cd.extracted as f64 / kg.stats.edge_count as f64 * 100.0 } else { 0.0 },
        cd.inferred,
        if kg.stats.edge_count > 0 { cd.inferred as f64 / kg.stats.edge_count as f64 * 100.0 } else { 0.0 },
        cd.ambiguous,
        if kg.stats.edge_count > 0 { cd.ambiguous as f64 / kg.stats.edge_count as f64 * 100.0 } else { 0.0 },
    ));

    report.push_str("\n---\n\n*Generated by Graphify Pro*\n");
    report
}

/// Export a Mermaid.js architecture diagram (call-flow / class diagram).
pub fn export_mermaid(kg: &KnowledgeGraph, communities: &[Community]) -> String {
    let mut diagram = String::from("# Architecture Call-Flow Diagram\n\n");
    diagram.push_str("```mermaid\ngraph TD\n");

    // Color scheme for node types
    for node in &kg.nodes {
        let (shape, close) = match node.node_type {
            graphify_core::node::NodeType::Class => ("([", "])"),
            graphify_core::node::NodeType::Interface => ("{{", "}}"),
            graphify_core::node::NodeType::Function => ("[", "]"),
            graphify_core::node::NodeType::Enum => ("((", "))"),
            graphify_core::node::NodeType::File => ("[/", "/]"),
            _ => ("[", "]"),
        };
        let clean_label = node.label.replace('"', "'");
        let safe_id = node.id.replace(['.', '-'], "_").replace(":", "_");
        let style = if node.is_god_node { " fill:#f96" } else { "" };
        diagram.push_str(&format!(
            "    {}{}[\"{}\"]{}{}\n",
            safe_id, shape, clean_label, close, style
        ));
    }

    // Add edges
    for edge in kg.edges.iter().take(100) {
        let label = match edge.relation {
            graphify_core::edge::EdgeRelation::Calls => "calls",
            graphify_core::edge::EdgeRelation::Contains => "contains",
            graphify_core::edge::EdgeRelation::Imports => "imports",
            graphify_core::edge::EdgeRelation::Inherits => "extends",
            graphify_core::edge::EdgeRelation::Implements => "implements",
            _ => "",
        };
        if label.is_empty() { continue; }
        let style = match edge.confidence {
            graphify_core::confidence::Confidence::Extracted => "",
            graphify_core::confidence::Confidence::Inferred => " -.- ",
            graphify_core::confidence::Confidence::Ambiguous => " -.- ",
        };
        let safe_src = edge.source.replace(['.', '-'], "_").replace(":", "_");
        let safe_tgt = edge.target.replace(['.', '-'], "_").replace(":", "_");
        if style.is_empty() {
            diagram.push_str(&format!("    {} -->|{}| {}\n", safe_src, label, safe_tgt));
        } else {
            diagram.push_str(&format!("    {}{}|{}| {}\n", safe_src, style, label, safe_tgt));
        }
    }

    // Community subgraphs
    for comm in communities.iter().take(8) {
        diagram.push_str(&format!("    subgraph {}[\"{} ({})\"]\n", comm.id, comm.label, comm.size));
        for node_id in &comm.nodes {
            diagram.push_str(&format!("        {}\n", node_id));
        }
        diagram.push_str("    end\n");
    }

    diagram.push_str("```\n");
    diagram
}

/// Export graph to SVG with nodes and edges.
pub fn export_svg(kg: &KnowledgeGraph) -> String {
    use std::collections::HashMap;
    use std::fmt::Write;
    let mut svg = String::new();
    svg.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 1000 700\">\n");
    svg.push_str("  <style>\n");
    svg.push_str("    .node { fill: #58a6ff; stroke: #30363d; stroke-width: 0.5; }\n");
    svg.push_str("    .god { fill: #f78166; stroke: #f78166; stroke-width: 1; }\n");
    svg.push_str("    .edge { stroke: #30363d; stroke-width: 0.3; }\n");
    svg.push_str("    .label { font-family: monospace; font-size: 5px; fill: #8b949e; }\n");
    svg.push_str("  </style>\n");

    // Circular layout for nodes
    let n = kg.nodes.len().max(1);
    let cx = 500.0;
    let cy = 350.0;
    let radius = 280.0;
    let mut positions: HashMap<String, (f64, f64)> = HashMap::new();

    for (i, node) in kg.nodes.iter().enumerate() {
        let angle = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
        let x = cx + radius * angle.cos();
        let y = cy + radius * angle.sin();
        positions.insert(node.id.clone(), (x, y));
    }

    // Draw edges
    for edge in &kg.edges {
        if let (Some(&(x1, y1)), Some(&(x2, y2))) = (positions.get(&edge.source), positions.get(&edge.target)) {
            let opacity = if edge.confidence == graphify_core::confidence::Confidence::Inferred { "0.3" } else { "0.5" };
            let _ = writeln!(svg, "  <line x1='{x1}' y1='{y1}' x2='{x2}' y2='{y2}' class='edge' opacity='{op}'/>", x1=x1, y1=y1, x2=x2, y2=y2, op=opacity);
        }
    }

    // Draw nodes
    for node in kg.nodes.iter() {
        if let Some(&(x, y)) = positions.get(&node.id) {
            let cls = if node.is_god_node { "god" } else { "node" };
            let r = if node.is_god_node { 5.0 } else { 2.5 };
            let label = node.label.chars().take(15).collect::<String>().replace('"', "'");
            let _ = writeln!(svg, "  <circle cx='{x}' cy='{y}' r='{r}' class='{cls}'/><text x='{tx}' y='{ty}' class='label'>{label}</text>",
                x = x, y = y, r = r, cls = cls,
                tx = x + 6.0, ty = y + 1.5,
                label = label);
        }
    }
    svg.push_str("</svg>\n");
    svg
}
/// Export graph as Neo4j-compatible CSV files (nodes.csv + relationships.csv).
pub fn export_neo4j_csv(kg: &KnowledgeGraph, output_dir: &Path) -> Result<(), anyhow::Error> {
    std::fs::create_dir_all(output_dir)?;

    // Nodes CSV: id,label,type,language,isGodNode,sourceFile
    let nodes_path = output_dir.join("neo4j_nodes.csv");
    let mut wtr = csv::Writer::from_path(&nodes_path)?;
    wtr.write_record(["nodeId:ID", "label", "type", "language", "isGodNode:boolean", "sourceFile"])?;
    for node in &kg.nodes {
        let type_label = node.node_type.label().to_string();
        let lang = node.language.clone().unwrap_or_default();
        let is_god = if node.is_god_node { "true".to_string() } else { "false".to_string() };
        let src_file = node.source_file.clone().unwrap_or_default();
        wtr.write_record([
            &node.id,
            &node.label,
            &type_label,
            &lang,
            &is_god,
            &src_file,
        ])?;
    }
    wtr.flush()?;

    // Relationships CSV: source,target,relation,confidence
    let edges_path = output_dir.join("neo4j_relationships.csv");
    let mut wtr = csv::Writer::from_path(&edges_path)?;
    wtr.write_record([":START_ID", ":END_ID", ":TYPE", "confidence", "weight:float"])?;
    for edge in &kg.edges {
        let rel_label = edge.relation.label().to_string();
        let conf_str = format!("{:?}", edge.confidence);
        let weight_str = edge.weight.to_string();
        wtr.write_record([
            &edge.source,
            &edge.target,
            &rel_label,
            &conf_str,
            &weight_str,
        ])?;
    }
    wtr.flush()?;

    println!("📦 Neo4j CSV exported to {}", output_dir.display());
    Ok(())
}

/// Export graph as Obsidian-compatible wiki-links (markdown vault).
pub fn export_obsidian(kg: &KnowledgeGraph, communities: &[Community], output_dir: &Path) -> Result<(), anyhow::Error> {
    std::fs::create_dir_all(output_dir)?;

    // Node index with wiki-links
    let node_map: HashMap<&str, &graphify_core::node::GraphNode> = kg.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // Community pages
    for comm in communities {
        let safe_name = comm.label.replace(['/', '\\', ':', '?', '*', '"', '<', '>', '|'], "_");
        let page_path = output_dir.join(format!("{}.md", safe_name));
        let mut page = String::new();
        page.push_str(&format!("# {} (Community)\n\n", comm.label));
        page.push_str(&format!("**Size:** {} nodes\n\n", comm.size));
        if let Some(ref desc) = comm.description {
            page.push_str(&format!("> {}\n\n", desc));
        }
        page.push_str("## Nodes\n\n");
        for node_id in &comm.nodes {
            if let Some(node) = node_map.get(node_id.as_str()) {
                let node_type = node.node_type.label();
                page.push_str(&format!("- **[[{}]]** ({})\n", node.label, node_type));
            }
        }
        std::fs::write(&page_path, page)?;
    }

    // Hub/GOD page
    let hub_path = output_dir.join("GOD_NODES.md");
    let mut hub_page = String::from("# 🏛️ God Nodes\n\nArchitectural hubs of the codebase.\n\n");
    let mut node_degrees: Vec<(&graphify_core::node::GraphNode, usize)> = Vec::new();
    let mut degree_map: HashMap<&str, usize> = HashMap::new();
    for edge in &kg.edges {
        *degree_map.entry(edge.source.as_str()).or_default() += 1;
        *degree_map.entry(edge.target.as_str()).or_default() += 1;
    }
    for node in &kg.nodes {
        let deg = degree_map.get(node.id.as_str()).copied().unwrap_or(0);
        node_degrees.push((node, deg));
    }
    node_degrees.sort_by_key(|(_, d)| std::cmp::Reverse(*d));
    for (node, deg) in node_degrees.iter().take(20) {
        hub_page.push_str(&format!("- **[[{}]]** — {} connections\n", node.label, deg));
    }
    std::fs::write(&hub_path, hub_page)?;

    // Per-node pages
    for node in &kg.nodes {
        let safe_name = node.label.replace(['/', '\\', ':', '?', '*', '"', '<', '>', '|'], "_");
        let page_path = output_dir.join(format!("{}.md", safe_name));
        let mut page = String::new();
        page.push_str(&format!("# {}\n\n", node.label));
        page.push_str(&format!("- **Type:** {}\n", node.node_type.label()));
        if let Some(ref lang) = node.language {
            page.push_str(&format!("- **Language:** {}\n", lang));
        }
        if let Some(ref file) = node.source_file {
            page.push_str(&format!("- **File:** `{}`\n", file));
        }
        if node.is_god_node {
            page.push_str("- **🏛️ God Node**\n");
        }

        // Incoming edges
        let incoming: Vec<_> = kg.edges.iter().filter(|e| e.target == node.id).collect();
        if !incoming.is_empty() {
            page.push_str("\n## Incoming\n\n");
            for edge in incoming.iter().take(15) {
                if let Some(src) = node_map.get(edge.source.as_str()) {
                    page.push_str(&format!("- [[{}]] → {} ({})\n", src.label, node.label, edge.relation.label()));
                }
            }
        }

        // Outgoing edges
        let outgoing: Vec<_> = kg.edges.iter().filter(|e| e.source == node.id).collect();
        if !outgoing.is_empty() {
            page.push_str("\n## Outgoing\n\n");
            for edge in outgoing.iter().take(15) {
                if let Some(tgt) = node_map.get(edge.target.as_str()) {
                    page.push_str(&format!("- {} → [[{}]] ({})\n", node.label, tgt.label, edge.relation.label()));
                }
            }
        }

        std::fs::write(&page_path, page)?;
    }

    // Index page
    let index_path = output_dir.join("INDEX.md");
    let mut index = String::from("# Knowledge Graph Index\n\n");
    index.push_str(&format!("**Nodes:** {} | **Edges:** {} | **Communities:** {}\n\n", kg.stats.node_count, kg.stats.edge_count, communities.len()));
    index.push_str("## Communities\n\n");
    for comm in communities {
        let safe_name = comm.label.replace(['/', '\\', ':', '?', '*', '"', '<', '>', '|'], "_");
        index.push_str(&format!("- [[{}]] ({} nodes)\n", safe_name, comm.size));
    }
    index.push_str("\n## God Nodes\n\nSee [[GOD_NODES]]\n");
    std::fs::write(&index_path, index)?;

    println!("📓 Obsidian vault exported to {}", output_dir.display());
    Ok(())
}

/// Export all artifacts to the output directory.
pub fn export_all(
    kg: &KnowledgeGraph,
    communities: &[Community],
    god_nodes: &[(String, String, usize)],
    metrics: &[NodeMetrics],
    output_dir: &Path,
) -> Result<(), anyhow::Error> {
    std::fs::create_dir_all(output_dir)?;

    // graph.json
    let json_path = output_dir.join("graph.json");
    std::fs::write(&json_path, export_json(kg)?)?;

    // graph.html
    let html_path = output_dir.join("graph.html");
    std::fs::write(&html_path, export_html(kg, communities))?;

    // GRAPH_REPORT.md
    let report_path = output_dir.join("GRAPH_REPORT.md");
    std::fs::write(&report_path, export_markdown(kg, communities, god_nodes, metrics))?;

    // graph.mermaid.md (Mermaid call-flow diagram)
    let mermaid_path = output_dir.join("graph.mermaid.md");
    std::fs::write(&mermaid_path, export_mermaid(kg, communities))?;

    // graph.svg (architectural diagram)
    let svg_path = output_dir.join("graph.svg");
    std::fs::write(&svg_path, export_svg(kg))?;

    // Neo4j CSV
    let neo4j_dir = output_dir.join("neo4j");
    if let Err(e) = export_neo4j_csv(kg, &neo4j_dir) {
        eprintln!("⚠️  Neo4j export failed: {}", e);
    }

    // Obsidian wiki
    let obsidian_dir = output_dir.join("obsidian");
    if let Err(e) = export_obsidian(kg, communities, &obsidian_dir) {
        eprintln!("⚠️  Obsidian export failed: {}", e);
    }

    println!(
        "📊 Exported to {}: graph.json, graph.html, GRAPH_REPORT.md, graph.mermaid.md, graph.svg, neo4j/, obsidian/",
        output_dir.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphify_core::node::{GraphNode, NodeType};
    use graphify_core::edge::{GraphEdge, EdgeRelation};

    fn make_test_kg() -> KnowledgeGraph {
        KnowledgeGraph {
            schema_version: "2.0".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            project_root: "/test/project".into(),
            metadata: graphify_core::GraphMetadata {
                project_name: Some("test-project".into()),
                primary_language: Some("Rust".into()),
                languages: vec!["Rust".into()],
                total_files: 5,
                total_lines: 500,
                git_branch: None,
                git_commit: None,
            },
            nodes: vec![
                GraphNode::new("n1", "UserService", NodeType::Class),
                GraphNode::new("n2", "get_user", NodeType::Function),
            ],
            edges: vec![
                GraphEdge::new("n1", "n2", EdgeRelation::Contains),
            ],
            hyperedges: vec![],
            communities: vec![],
            stats: GraphStats {
                node_count: 2,
                edge_count: 1,
                hyperedge_count: 0,
                community_count: 0,
                avg_degree: 1.0,
                density: 0.5,
                connected_components: 1,
                is_connected: true,
                confidence_distribution: graphify_core::ConfidenceDistribution {
                    extracted: 1,
                    inferred: 0,
                    ambiguous: 0,
                },
            },
        }
    }

    #[test]
    fn test_export_json() {
        let kg = make_test_kg();
        let json = export_json(&kg).unwrap();
        assert!(json.contains("UserService"));
        assert!(json.contains("schema_version"));
    }

    #[test]
    fn test_export_html() {
        let kg = make_test_kg();
        let communities = vec![];
        let html = export_html(&kg, &communities);
        assert!(html.contains("Graphify Pro"));
        assert!(html.contains("d3.v7.min.js"), "HTML should include D3.js");
    }

    #[test]
    fn test_export_markdown() {
        let kg = make_test_kg();
        let god_nodes = vec![("n1".into(), "UserService".into(), 5)];
        let metrics = vec![];
        let md = export_markdown(&kg, &[], &god_nodes, &metrics);
        assert!(md.contains("UserService"));
        assert!(md.contains("God Nodes"));
    }
}
