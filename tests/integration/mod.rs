// CROSS-DEP: needs root Cargo.toml [package] + [[test]] + [dev-dependencies] to compile.
// Boss will add these during final integration.

mod helpers;

// dev2 modules (they write these):
// mod test_cli;
// mod test_query_loop;
// mod test_tools;

// dev5 modules:
mod test_commands;
mod test_mcp;
mod test_session;
