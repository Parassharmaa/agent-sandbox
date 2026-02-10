use std::collections::HashMap;

use super::ast::Command;

/// Shell environment: variables, functions, positional parameters.
#[derive(Debug, Clone)]
pub struct ShellEnv {
    /// Local shell variables
    vars: HashMap<String, String>,
    /// Exported variables (also in vars)
    exports: HashMap<String, bool>,
    /// Positional parameters ($1, $2, ...)
    pub positional: Vec<String>,
    /// Last exit status ($?)
    pub last_status: i32,
    /// Shell functions
    pub functions: HashMap<String, Command>,
    /// Local variable scopes (for function-local vars)
    local_stack: Vec<HashMap<String, Option<String>>>,
}

impl ShellEnv {
    pub fn new() -> Self {
        let mut env = ShellEnv {
            vars: HashMap::new(),
            exports: HashMap::new(),
            positional: Vec::new(),
            last_status: 0,
            functions: HashMap::new(),
            local_stack: Vec::new(),
        };

        // Import environment variables
        for (key, value) in std::env::vars() {
            env.vars.insert(key.clone(), value);
            env.exports.insert(key, true);
        }

        env
    }

    /// Get a variable value.
    pub fn get(&self, name: &str) -> Option<&str> {
        // Special variables
        match name {
            "?" => return None, // Handled specially by caller
            "#" => return None, // Handled specially by caller
            "@" | "*" => return None,
            "0" => return Some("sh"),
            _ => {}
        }

        // Check if positional parameter
        if let Ok(n) = name.parse::<usize>() {
            if n >= 1 && n <= self.positional.len() {
                return self.positional.get(n - 1).map(|s| s.as_str());
            }
            return None;
        }

        self.vars.get(name).map(|s| s.as_str())
    }

    /// Set a variable value.
    pub fn set(&mut self, name: &str, value: &str) {
        // If we're in a local scope, track the old value
        if let Some(scope) = self.local_stack.last_mut() {
            if !scope.contains_key(name) {
                scope.insert(name.to_string(), self.vars.get(name).cloned());
            }
        }
        self.vars.insert(name.to_string(), value.to_string());
    }

    /// Unset a variable.
    pub fn unset(&mut self, name: &str) {
        self.vars.remove(name);
        self.exports.remove(name);
    }

    /// Mark a variable as exported.
    pub fn export(&mut self, name: &str, value: Option<&str>) {
        if let Some(v) = value {
            self.vars.insert(name.to_string(), v.to_string());
        }
        self.exports.insert(name.to_string(), true);
    }

    /// Check if exported.
    pub fn is_exported(&self, name: &str) -> bool {
        self.exports.contains_key(name)
    }

    /// Get all exported variables for child processes.
    pub fn exported_vars(&self) -> Vec<(String, String)> {
        self.exports
            .keys()
            .filter_map(|k| self.vars.get(k).map(|v| (k.clone(), v.clone())))
            .collect()
    }

    /// Get the number of positional parameters.
    pub fn num_positional(&self) -> usize {
        self.positional.len()
    }

    /// Get all positional params joined by space ($*) or as separate strings ($@).
    pub fn all_positional(&self) -> &[String] {
        &self.positional
    }

    /// Shift positional parameters left by n.
    pub fn shift(&mut self, n: usize) {
        if n <= self.positional.len() {
            self.positional = self.positional[n..].to_vec();
        }
    }

    /// Push a new local variable scope.
    pub fn push_local_scope(&mut self) {
        self.local_stack.push(HashMap::new());
    }

    /// Pop a local variable scope, restoring old values.
    pub fn pop_local_scope(&mut self) {
        if let Some(scope) = self.local_stack.pop() {
            for (name, old_value) in scope {
                match old_value {
                    Some(v) => {
                        self.vars.insert(name, v);
                    }
                    None => {
                        self.vars.remove(&name);
                    }
                }
            }
        }
    }

    /// Declare a variable as local (only meaningful inside a function).
    pub fn declare_local(&mut self, name: &str) {
        if let Some(scope) = self.local_stack.last_mut() {
            if !scope.contains_key(name) {
                scope.insert(name.to_string(), self.vars.get(name).cloned());
            }
        }
    }
}
