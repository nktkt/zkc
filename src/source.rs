use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{Expr, FunctionDecl, Item, Program, Statement, format_callee};
use crate::error::{CompileError, CompileResult};
use crate::parser;
use crate::pretty::render_program;
use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyGraph {
    pub root: PathBuf,
    pub nodes: Vec<PathBuf>,
    pub edges: Vec<DependencyEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEdge {
    pub from: PathBuf,
    pub to: PathBuf,
    pub kind: DependencyKind,
    pub namespace: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyKind {
    Include,
    Import,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProgram {
    pub program: Program,
    pub graph: DependencyGraph,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdlibCatalog {
    pub root: PathBuf,
    pub modules: Vec<StdlibModule>,
    pub total_lines: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdlibModule {
    pub logical_path: String,
    pub absolute_path: PathBuf,
    pub line_count: usize,
}

pub fn resolve_program(path: impl AsRef<Path>) -> CompileResult<ResolvedProgram> {
    let mut resolver = Resolver::default();
    resolver.resolve_root(path.as_ref())
}

pub fn dependency_graph(path: impl AsRef<Path>) -> CompileResult<DependencyGraph> {
    resolve_program(path).map(|resolved| resolved.graph)
}

pub fn stdlib_catalog() -> CompileResult<StdlibCatalog> {
    let root = stdlib_root();
    let mut modules = Vec::new();
    collect_stdlib_modules(&root, &root, &mut modules)?;
    modules.sort_by(|lhs, rhs| lhs.logical_path.cmp(&rhs.logical_path));
    let total_lines = modules.iter().map(|module| module.line_count).sum();
    Ok(StdlibCatalog {
        root,
        modules,
        total_lines,
    })
}

impl DependencyGraph {
    pub fn to_json(&self) -> String {
        let mut out = String::from("{");
        push_field(&mut out, "root", &json_string(&display_path(&self.root)));
        out.push(',');
        push_field(
            &mut out,
            "nodes",
            &json_array(&self.nodes, |path| json_string(&display_path(path))),
        );
        out.push(',');
        push_field(
            &mut out,
            "edges",
            &json_array(&self.edges, dependency_edge_json),
        );
        out.push('}');
        out
    }
}

impl ResolvedProgram {
    pub fn to_json(&self) -> String {
        let mut out = String::from("{");
        push_field(&mut out, "graph", &self.graph.to_json());
        out.push(',');
        push_field(
            &mut out,
            "source",
            &json_string(&render_program(&self.program)),
        );
        out.push('}');
        out
    }
}

impl StdlibCatalog {
    pub fn to_json(&self) -> String {
        let mut out = String::from("{");
        push_field(&mut out, "root", &json_string(&display_path(&self.root)));
        out.push(',');
        push_field(&mut out, "total_modules", &self.modules.len().to_string());
        out.push(',');
        push_field(&mut out, "total_lines", &self.total_lines.to_string());
        out.push(',');
        push_field(
            &mut out,
            "modules",
            &json_array(&self.modules, stdlib_module_json),
        );
        out.push('}');
        out
    }
}

impl fmt::Display for DependencyGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "dependency graph for {}", display_path(&self.root))?;
        writeln!(f, "nodes:")?;
        if self.nodes.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for node in &self.nodes {
                writeln!(f, "  {}", display_path(node))?;
            }
        }
        writeln!(f, "edges:")?;
        if self.edges.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for edge in &self.edges {
                writeln!(
                    f,
                    "  {} -> {} [{}{}]",
                    display_path(&edge.from),
                    display_path(&edge.to),
                    edge.kind.label(),
                    render_namespace_hint(&edge.namespace)
                )?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for StdlibCatalog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "stdlib catalog at {}", display_path(&self.root))?;
        writeln!(
            f,
            "modules: {} ({} lines)",
            self.modules.len(),
            self.total_lines
        )?;
        if self.modules.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for module in &self.modules {
                writeln!(
                    f,
                    "  {}  ({} lines)",
                    module.logical_path, module.line_count
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct Resolver {
    stack: Vec<PathBuf>,
    expanded: HashSet<ExpandKey>,
    nodes: Vec<PathBuf>,
    edges: Vec<DependencyEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ExpandKey {
    path: PathBuf,
    namespace: String,
}

impl Resolver {
    fn resolve_root(&mut self, path: &Path) -> CompileResult<ResolvedProgram> {
        let root = self.normalize_path(path, None)?;
        trace_resolution(&format!("resolve_root {}", display_path(&root)));
        self.add_node(root.clone());
        self.stack.push(root.clone());
        self.expanded.insert(expand_key(&root, &[]));

        let source = read_file(&root)?;
        let program = parser::parse(&source).map_err(|err| annotate_error(err, &root))?;
        let items = self.resolve_items(&root, program.circuit.items, &[], false)?;
        self.stack.pop();

        Ok(ResolvedProgram {
            program: Program {
                circuit: crate::ast::Circuit {
                    name: program.circuit.name,
                    items,
                },
            },
            graph: DependencyGraph {
                root,
                nodes: self.nodes.clone(),
                edges: self.edges.clone(),
            },
        })
    }

    fn resolve_items(
        &mut self,
        owner: &Path,
        items: Vec<Item>,
        namespace: &[String],
        module_mode: bool,
    ) -> CompileResult<Vec<Item>> {
        let mut resolved = Vec::new();

        for item in items {
            match item {
                Item::Include(include) => {
                    let target = self.normalize_path(Path::new(&include.path), Some(owner))?;
                    trace_resolution(&format!(
                        "include {} -> {} [{}]",
                        display_path(owner),
                        display_path(&target),
                        format_callee(namespace)
                    ));
                    self.add_node(target.clone());
                    self.edges.push(DependencyEdge {
                        from: owner.to_path_buf(),
                        to: target.clone(),
                        kind: DependencyKind::Include,
                        namespace: format_callee(namespace),
                    });

                    if let Some(index) = self.stack.iter().position(|path| path == &target) {
                        let mut chain = self.stack[index..]
                            .iter()
                            .map(|path| display_path(path))
                            .collect::<Vec<_>>();
                        chain.push(display_path(&target));
                        return Err(CompileError::new(
                            include.span,
                            format!("circular include detected: {}", chain.join(" -> ")),
                        ));
                    }

                    let key = expand_key(&target, namespace);
                    if self.expanded.contains(&key) {
                        continue;
                    }

                    self.stack.push(target.clone());
                    self.expanded.insert(key);
                    let source = read_file(&target)?;
                    let parsed =
                        parser::parse_items(&source).map_err(|err| annotate_error(err, &target))?;
                    let nested = self.resolve_items(&target, parsed, namespace, module_mode)?;
                    self.stack.pop();
                    resolved.extend(nested);
                }
                Item::Import(import) => {
                    let target = self.normalize_path(Path::new(&import.path), Some(owner))?;
                    let next_namespace = extend_namespace(namespace, &import.alias);
                    trace_resolution(&format!(
                        "import {} -> {} as {}",
                        display_path(owner),
                        display_path(&target),
                        format_callee(&next_namespace)
                    ));
                    self.add_node(target.clone());
                    self.edges.push(DependencyEdge {
                        from: owner.to_path_buf(),
                        to: target.clone(),
                        kind: DependencyKind::Import,
                        namespace: format_callee(&next_namespace),
                    });

                    if let Some(index) = self.stack.iter().position(|path| path == &target) {
                        let mut chain = self.stack[index..]
                            .iter()
                            .map(|path| display_path(path))
                            .collect::<Vec<_>>();
                        chain.push(display_path(&target));
                        return Err(CompileError::new(
                            import.span,
                            format!("circular include detected: {}", chain.join(" -> ")),
                        ));
                    }

                    let key = expand_key(&target, &next_namespace);
                    if self.expanded.contains(&key) {
                        continue;
                    }

                    self.stack.push(target.clone());
                    self.expanded.insert(key);
                    let source = read_file(&target)?;
                    let parsed =
                        parser::parse_items(&source).map_err(|err| annotate_error(err, &target))?;
                    let nested = self.resolve_items(&target, parsed, &next_namespace, true)?;
                    self.stack.pop();
                    resolved.extend(nested);
                }
                Item::Function(function) => {
                    trace_resolution(&format!(
                        "function {} [{}]",
                        function.name,
                        format_callee(namespace)
                    ));
                    if namespace.is_empty() {
                        resolved.push(Item::Function(function));
                    } else {
                        resolved.push(Item::Function(namespace_function(function, namespace)));
                    }
                }
                Item::Input(input) => {
                    if module_mode {
                        return Err(CompileError::new(
                            input.span,
                            "imported modules may only contain functions, includes, and imports",
                        ));
                    }
                    resolved.push(Item::Input(input));
                }
                Item::Statement(statement) => {
                    if module_mode {
                        return Err(statement_span(&statement).map_or_else(
                            || {
                                CompileError::new(
                                    Span::default(),
                                    "imported modules may only contain functions, includes, and imports",
                                )
                            },
                            |span| {
                                CompileError::new(
                                    span,
                                    "imported modules may only contain functions, includes, and imports",
                                )
                            },
                        ));
                    }
                    resolved.push(Item::Statement(statement));
                }
            }
        }

        Ok(resolved)
    }

    fn normalize_path(&self, path: &Path, owner: Option<&Path>) -> CompileResult<PathBuf> {
        let raw = path.to_string_lossy();
        let base = if let Some(std_path) = raw.strip_prefix("@std/") {
            stdlib_root().join(std_path)
        } else if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(owner) = owner {
            owner.parent().unwrap_or_else(|| Path::new(".")).join(path)
        } else {
            path.to_path_buf()
        };

        fs::canonicalize(&base).map_err(|err| {
            CompileError::new(
                Span::default(),
                format!("failed to resolve `{}`: {err}", display_path(&base)),
            )
        })
    }

    fn add_node(&mut self, path: PathBuf) {
        if !self.nodes.contains(&path) {
            self.nodes.push(path);
        }
    }
}

fn read_file(path: &Path) -> CompileResult<String> {
    fs::read_to_string(path).map_err(|err| {
        CompileError::new(
            Span::default(),
            format!("failed to read `{}`: {err}", display_path(path)),
        )
    })
}

fn namespace_function(function: FunctionDecl, namespace: &[String]) -> FunctionDecl {
    let mut qualified = namespace.to_vec();
    qualified.push(function.name);
    FunctionDecl {
        name: format_callee(&qualified),
        params: function.params,
        return_type: function.return_type,
        body: namespace_expr(function.body, namespace),
        span: function.span,
    }
}

fn namespace_expr(expr: Expr, namespace: &[String]) -> Expr {
    match expr {
        Expr::Number { .. } | Expr::Bool { .. } | Expr::Ident { .. } => expr,
        Expr::Call { callee, args, span } => {
            let callee = if callee.len() == 1 && crate::builtins::contains(&callee[0]) {
                callee
            } else {
                qualify_callee(namespace, &callee)
            };
            Expr::Call {
                callee,
                args: args
                    .into_iter()
                    .map(|arg| namespace_expr(arg, namespace))
                    .collect(),
                span,
            }
        }
        Expr::Unary { op, expr, span } => Expr::Unary {
            op,
            expr: Box::new(namespace_expr(*expr, namespace)),
            span,
        },
        Expr::Binary { op, lhs, rhs, span } => Expr::Binary {
            op,
            lhs: Box::new(namespace_expr(*lhs, namespace)),
            rhs: Box::new(namespace_expr(*rhs, namespace)),
            span,
        },
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            span,
        } => Expr::IfElse {
            condition: Box::new(namespace_expr(*condition, namespace)),
            then_branch: Box::new(namespace_expr(*then_branch, namespace)),
            else_branch: Box::new(namespace_expr(*else_branch, namespace)),
            span,
        },
    }
}

fn qualify_callee(namespace: &[String], callee: &[String]) -> Vec<String> {
    let mut qualified = namespace.to_vec();
    qualified.extend(callee.iter().cloned());
    qualified
}

fn extend_namespace(namespace: &[String], segment: &str) -> Vec<String> {
    let mut next = namespace.to_vec();
    next.push(segment.to_string());
    next
}

fn expand_key(path: &Path, namespace: &[String]) -> ExpandKey {
    ExpandKey {
        path: path.to_path_buf(),
        namespace: format_callee(namespace),
    }
}

fn statement_span(statement: &Statement) -> Option<Span> {
    Some(match statement {
        Statement::Let(stmt) => stmt.span,
        Statement::Constrain(stmt) => stmt.span,
        Statement::Expose(stmt) => stmt.span,
    })
}

fn trace_resolution(message: &str) {
    if std::env::var_os("ZKC_TRACE_RESOLUTION").is_some() {
        eprintln!("[resolve] {message}");
    }
}

fn annotate_error(err: CompileError, path: &Path) -> CompileError {
    CompileError::new(
        err.span,
        format!("{} in `{}`", err.message, display_path(path)),
    )
}

fn dependency_edge_json(edge: &DependencyEdge) -> String {
    format!(
        "{{\"from\":{},\"to\":{},\"kind\":{},\"namespace\":{}}}",
        json_string(&display_path(&edge.from)),
        json_string(&display_path(&edge.to)),
        json_string(edge.kind.label()),
        json_string(&edge.namespace)
    )
}

fn stdlib_module_json(module: &StdlibModule) -> String {
    format!(
        concat!(
            "{{",
            "\"logical_path\":{},",
            "\"absolute_path\":{},",
            "\"line_count\":{}",
            "}}"
        ),
        json_string(&module.logical_path),
        json_string(&display_path(&module.absolute_path)),
        module.line_count
    )
}

fn stdlib_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("stdlib")
}

fn collect_stdlib_modules(
    root: &Path,
    directory: &Path,
    modules: &mut Vec<StdlibModule>,
) -> CompileResult<()> {
    let entries = fs::read_dir(directory).map_err(|err| {
        CompileError::new(
            Span::default(),
            format!("failed to read `{}`: {err}", display_path(directory)),
        )
    })?;

    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|err| {
                CompileError::new(
                    Span::default(),
                    format!(
                        "failed to read entry under `{}`: {err}",
                        display_path(directory)
                    ),
                )
            })?
            .path();
        paths.push(path);
    }
    paths.sort();

    for path in paths {
        if path.is_dir() {
            collect_stdlib_modules(root, &path, modules)?;
            continue;
        }
        if path.extension().is_none_or(|ext| ext != "zk") {
            continue;
        }

        let logical_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let line_count = fs::read_to_string(&path)
            .map_err(|err| {
                CompileError::new(
                    Span::default(),
                    format!("failed to read `{}`: {err}", display_path(&path)),
                )
            })?
            .lines()
            .count();

        modules.push(StdlibModule {
            logical_path,
            absolute_path: path,
            line_count,
        });
    }

    Ok(())
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

impl DependencyKind {
    fn label(self) -> &'static str {
        match self {
            Self::Include => "include",
            Self::Import => "import",
        }
    }
}

fn render_namespace_hint(namespace: &str) -> String {
    if namespace.is_empty() {
        String::new()
    } else {
        format!(", namespace={namespace}")
    }
}

fn push_field(out: &mut String, key: &str, value: &str) {
    out.push_str(&json_string(key));
    out.push(':');
    out.push_str(value);
}

fn json_array<T>(items: &[T], encode: fn(&T) -> String) -> String {
    let mut out = String::from("[");
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&encode(item));
    }
    out.push(']');
    out
}

fn json_string(input: &str) -> String {
    let mut out = String::from("\"");
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}
