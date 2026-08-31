//! Feature-gated integration stubs for future scientific crates.
//!
//! `tpt-sci-reaction-network` and `tpt-sci-ode` are not yet published (per
//! `todo.md`). These modules provide only the conversion boundary — a stable API
//! plus local placeholder types — so the `sci-reaction-network-integration` and
//! `sci-ode-integration` feature flags compile and link today.

#[cfg(feature = "sci-reaction-network-integration")]
pub mod reaction_network {
    //! Stub conversion into a future `tpt_sci_reaction_network` type.
    use alloc::string::String;
    use alloc::vec::Vec;

    use crate::model::Sbml;
    use crate::SbmlError;

    /// A node in the placeholder reaction network (a species id).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct NetworkNode {
        /// Species id.
        pub species: String,
        /// Compartment id (if known).
        pub compartment: Option<String>,
    }

    /// A directed edge in the placeholder reaction network (a reaction).
    #[derive(Debug, Clone, PartialEq)]
    pub struct NetworkEdge {
        /// Reaction id.
        pub reaction: String,
        /// Reactant species ids.
        pub reactants: Vec<String>,
        /// Product species ids.
        pub products: Vec<String>,
        /// Infix representation of the kinetic law (empty if absent).
        pub kinetic_law: String,
        /// Whether the reaction is reversible.
        pub reversible: bool,
    }

    /// Placeholder for a future `tpt_sci_reaction_network::ReactionNetwork`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct ReactionNetworkStub {
        /// Compartment ids.
        pub compartments: Vec<String>,
        /// Species nodes.
        pub nodes: Vec<NetworkNode>,
        /// Reaction edges.
        pub edges: Vec<NetworkEdge>,
    }

    /// Build a placeholder reaction network from a parsed SBML model.
    pub fn to_reaction_network(sbml: &Sbml) -> Result<ReactionNetworkStub, SbmlError> {
        let m = &sbml.model;
        let compartments = m.compartments.iter().map(|c| c.id.clone()).collect();
        let nodes = m
            .species
            .iter()
            .map(|s| NetworkNode {
                species: s.id.clone(),
                compartment: Some(s.compartment.clone()),
            })
            .collect();
        let edges = m
            .reactions
            .iter()
            .map(|r| NetworkEdge {
                reaction: r.id.clone(),
                reactants: r.reactants.iter().map(|sr| sr.species.clone()).collect(),
                products: r.products.iter().map(|sr| sr.species.clone()).collect(),
                kinetic_law: r
                    .kinetic_law
                    .as_ref()
                    .and_then(|kl| kl.math.as_ref())
                    .map(|math| math.to_infix_string())
                    .unwrap_or_default(),
                reversible: r.reversible,
            })
            .collect();
        Ok(ReactionNetworkStub { compartments, nodes, edges })
    }

    impl Sbml {
        /// Convert this model into a [`ReactionNetworkStub`].
        pub fn to_reaction_network(&self) -> Result<ReactionNetworkStub, SbmlError> {
            to_reaction_network(self)
        }
    }
}

#[cfg(feature = "sci-ode-integration")]
pub mod ode {
    //! Stub conversion into a future `tpt_sci_ode` type.
    use alloc::format;
    use alloc::string::String;
    use alloc::vec::Vec;

    use crate::model::Sbml;
    use crate::SbmlError;

    /// A single ODE: `d(state)/dt = rhs`.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Ode {
        /// State variable (species or parameter id).
        pub variable: String,
        /// Infix representation of the right-hand side.
        pub rhs: String,
    }

    /// Placeholder for a future `tpt_sci_ode::System`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct OdeSystemStub {
        /// State variables (species ids).
        pub states: Vec<String>,
        /// ODEs governing the states.
        pub equations: Vec<Ode>,
    }

    /// Build a placeholder ODE system from a parsed SBML model. Each reaction
    /// kinetic law contributes an ODE for its reactant/product species.
    pub fn to_ode_system(sbml: &Sbml) -> Result<OdeSystemStub, SbmlError> {
        let m = &sbml.model;
        let states = m.species.iter().map(|s| s.id.clone()).collect();
        let mut equations = Vec::new();
        for r in &m.reactions {
            if let Some(kl) = &r.kinetic_law {
                if let Some(math) = &kl.math {
                    let rhs = math.to_infix_string();
                    for sr in &r.products {
                        equations
                            .push(Ode { variable: sr.species.clone(), rhs: format!("+({rhs})") });
                    }
                    for sr in &r.reactants {
                        equations
                            .push(Ode { variable: sr.species.clone(), rhs: format!("-({rhs})") });
                    }
                }
            }
        }
        Ok(OdeSystemStub { states, equations })
    }

    impl Sbml {
        /// Convert this model into an [`OdeSystemStub`].
        pub fn to_ode_system(&self) -> Result<OdeSystemStub, SbmlError> {
            to_ode_system(self)
        }
    }
}
