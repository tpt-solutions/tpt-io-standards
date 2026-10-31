//! SED-ML summary: the models, simulations, tasks and outputs of a simulation experiment description.
//!
//! This reads the structure that ties a run together (which model, which simulation, which task links them,
//! and which outputs are plotted or reported). It does not run simulations and does not read the numeric
//! details of each simulation beyond its kind and algorithm.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::xml;

use crate::OmexError;

/// The summary of one SED-ML document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SedMl {
    /// The models, from `listOfModels`.
    pub models: Vec<SedModel>,
    /// The simulations, from `listOfSimulations`, whatever their kind.
    pub simulations: Vec<SedSimulation>,
    /// The tasks, from `listOfTasks`.
    pub tasks: Vec<SedTask>,
    /// The ids of the outputs (plots and reports), from `listOfOutputs`.
    pub outputs: Vec<String>,
}

/// A model reference: its id, the language URI, and the source file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SedModel {
    /// The model id.
    pub id: String,
    /// The language URI, such as `urn:sedml:language:sbml`.
    pub language: String,
    /// The source, usually a file name in the archive.
    pub source: String,
}

/// A simulation: its id, its element kind, and the algorithm's KiSAO id when given.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SedSimulation {
    /// The simulation id.
    pub id: String,
    /// The element name, such as `uniformTimeCourse` or `oneStep`.
    pub kind: String,
    /// The algorithm's KiSAO identifier, such as `KISAO:0000019`.
    pub algorithm: Option<String>,
}

/// A task: one model run with one simulation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SedTask {
    /// The task id.
    pub id: String,
    /// The id of the model it runs.
    pub model_ref: String,
    /// The id of the simulation it uses.
    pub simulation_ref: String,
}

fn attr(el: &xml::Element, name: &str) -> Result<String, OmexError> {
    el.attr(name)
        .map(|s| s.to_string())
        .ok_or_else(|| OmexError::Invalid(alloc::format!("<{}> has no {name} attribute", el.name)))
}

/// Summarise a SED-ML document.
pub fn parse_sedml(input: &[u8]) -> Result<SedMl, OmexError> {
    let root = xml::parse(input)?;
    if root.local_name() != "sedML" {
        return Err(OmexError::Invalid("root element is not <sedML>".to_string()));
    }
    let mut doc = SedMl::default();
    if let Some(list) = root.child("listOfModels") {
        for m in list.children_named("model") {
            doc.models.push(SedModel {
                id: attr(m, "id")?,
                language: attr(m, "language")?,
                source: attr(m, "source")?,
            });
        }
    }
    if let Some(list) = root.child("listOfSimulations") {
        for s in &list.children {
            doc.simulations.push(SedSimulation {
                id: attr(s, "id")?,
                kind: s.local_name().to_string(),
                algorithm: s
                    .child("algorithm")
                    .and_then(|a| a.attr("kisaoID"))
                    .map(|s| s.to_string()),
            });
        }
    }
    if let Some(list) = root.child("listOfTasks") {
        for t in list.children_named("task") {
            doc.tasks.push(SedTask {
                id: attr(t, "id")?,
                model_ref: attr(t, "modelReference")?,
                simulation_ref: attr(t, "simulationReference")?,
            });
        }
    }
    if let Some(list) = root.child("listOfOutputs") {
        for o in &list.children {
            doc.outputs.push(attr(o, "id")?);
        }
    }
    Ok(doc)
}
