// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use std::collections::BTreeMap;

use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Attribute, Cell, Color, Table,
};
use kythera_lib::{ExecutionEvent, Method, TestResult, TestResultType, WasmActor};

/// Gas report for the tested contracts.
#[derive(Default, Debug)]
pub struct GasReport<'a> {
    reports: BTreeMap<&'a WasmActor, ActorInfo<'a>>,
}

#[derive(Debug, Default)]
/// Actor method calls information
/// TODO: calculate actor deployment gas consumption.
pub struct ActorInfo<'a> {
    methods: BTreeMap<&'a Method, Vec<u64>>,
}

impl<'a> GasReport<'a> {
    /// Analyze a set of [`TestResult`]s for a target Actor.
    pub fn analyze(&mut self, actor: &'a WasmActor, test_results: &[&TestResult]) {
        let mut info = match self.reports.remove(actor) {
            Some(info) => info,
            None => ActorInfo::default(),
        };

        for result in test_results {
            let apply_ret = match result.ret() {
                TestResultType::Passed(apply_ret) | TestResultType::Failed(apply_ret) => apply_ret,
                TestResultType::Erred(_) => {
                    continue;
                }
            };

            // Get the Gas consumption by each Call of the Target actor.
            let mut stack = vec![];
            for trace in &apply_ret.exec_trace {
                match trace {
                    ExecutionEvent::GasCharge(gas_charge) => {
                        // Add this gas charge to the total gas charge by the main method.
                        // There is a `GasCharge` before any `Call` so we skip it.
                        let (_method, total_gas_charge) = match stack.get_mut(0) {
                            Some(e) => e,
                            None => continue,
                        };
                        *total_gas_charge =
                            *total_gas_charge + gas_charge.compute_gas.as_milligas();
                    }

                    ExecutionEvent::Call { method, .. } => {
                        stack.push((method, 0));
                    }
                    ExecutionEvent::CallReturn(_, _) | ExecutionEvent::CallError(_) => {
                        let (method_num, total_gas_charge) =
                            stack.pop().expect("A call return should match a Call");
                        // If stack is empty we reached the main Method call.
                        if !stack.is_empty() {
                            continue;
                        }
                        // If the method called was from the target actor,
                        // we create a new call on `GasInfo` with the totals of gas charge.
                        let Some(method) = actor
                                .abi()
                                .methods()
                                .iter()
                                .find(|a| a.number() == *method_num) else {
                            continue;
                        };

                        let mut gas_info = match info.methods.remove(method) {
                            Some(gi) => gi,
                            None => vec![],
                        };
                        gas_info.push(total_gas_charge);
                        info.methods.insert(method, gas_info);
                    }
                    _ => {}
                }
            }
        }
        self.reports.insert(actor, info);
    }

    /// Finalize the Report and convert into a printable Table.
    pub fn finalize(self) -> Vec<Table> {
        let mut tables = vec![];

        for (actor, contract_info) in self.reports {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.apply_modifier(UTF8_ROUND_CORNERS);
            table.set_header(vec![Cell::new(format!("{} contract", actor.name()))
                .add_attribute(Attribute::Bold)
                .fg(Color::Green)]);
            table.add_row(vec![
                Cell::new("Function Name")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Magenta),
                Cell::new("min")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Green),
                Cell::new("avg")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Yellow),
                Cell::new("median")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Yellow),
                Cell::new("max")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Red),
                Cell::new("# calls").add_attribute(Attribute::Bold),
            ]);
            for (method, mut calls) in contract_info.methods {
                calls.sort_unstable();
                let min = calls.last().copied().unwrap_or_default();
                let max = calls.last().copied().unwrap_or_default();

                let mean = {
                    if calls.is_empty() {
                        0f64
                    } else {
                        calls.iter().copied().fold(0, |sum, val| sum + val) as f64
                            / calls.len() as f64
                    }
                };

                let median = {
                    if calls.is_empty() {
                        0u64
                    } else {
                        let len = calls.len();
                        let mid = len / 2;
                        if len % 2 == 0 {
                            (calls[mid - 1] + calls[mid]) / 2u64
                        } else {
                            calls[mid]
                        }
                    }
                };
                table.add_row(vec![
                    Cell::new(method.name()).add_attribute(Attribute::Bold),
                    Cell::new(min.to_string()).fg(Color::Green),
                    Cell::new(mean.to_string()).fg(Color::Yellow),
                    Cell::new(median.to_string()).fg(Color::Yellow),
                    Cell::new(max.to_string()).fg(Color::Red),
                    Cell::new(calls.len().to_string()),
                ]);
            }
            tables.push(table);
        }
        tables
    }
}
