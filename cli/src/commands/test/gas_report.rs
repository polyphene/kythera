// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use std::collections::BTreeMap;

use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Attribute, Cell, Color, Table,
};
use kythera_lib::{DeployedActor, ExecutionEvent, Method, Payload, TestResult, TestResultType};

/// Gas report for the tested contracts.
#[derive(Default, Debug)]
pub struct GasReport {
    reports: BTreeMap<DeployedActor, ActorInfo>,
}

#[derive(Debug, Default)]
/// Actor method calls information
/// TODO: calculate actor deployment gas consumption.
pub struct ActorInfo {
    methods: BTreeMap<Method, Vec<u64>>,
}

/// A Method and its gas cost.
struct MethodCost {
    gas_cost: u64,
    num: u64,
    at: u64,
}

impl GasReport {
    /// Analyze a set of [`TestResult`]s for a target Actor.
    pub fn analyze(&mut self, actor: DeployedActor, test_results: &[TestResult]) {
        let mut info = match self.reports.remove(&actor) {
            Some(info) => info,
            None => ActorInfo::default(),
        };
        let actor_id = match actor.address().payload() {
            Payload::ID(id) => id,
            _ => panic!("DeployedActor address payload should be an Id"),
        };

        for result in test_results {
            let apply_ret = match result.ret() {
                TestResultType::Passed(apply_ret) | TestResultType::Failed(apply_ret) => apply_ret,
                TestResultType::Erred(_) => {
                    continue;
                }
            };

            // Get the Gas consumption by each Call of the Target actor.
            let mut stack: Vec<MethodCost> = vec![];
            for trace in &apply_ret.exec_trace {
                match trace {
                    ExecutionEvent::GasCharge(gas_charge) => {
                        // Add this gas charge to the total gas charge of the current method.
                        // There is a `GasCharge` before any `Call` so we skip it.
                        let method = match stack.last_mut() {
                            Some(e) => e,
                            None => continue,
                        };
                        method.gas_cost += gas_charge.compute_gas.as_milligas();
                    }

                    ExecutionEvent::Call { method, to, .. } => {
                        let to_id = match to.payload() {
                            Payload::ID(id) => id,
                            _ => panic!("Call to address payload should be an Id"),
                        };
                        stack.push(MethodCost {
                            gas_cost: 0,
                            num: *method,
                            at: *to_id,
                        });
                    }
                    ExecutionEvent::CallReturn(_, _) | ExecutionEvent::CallError(_) => {
                        let method_return = stack.pop().expect("A CallReturn should match a Call");
                        // If stack is empty we reached the main Method call.
                        // If the stack is not empty we keep summing the gas totals.
                        if let Some(previous) = stack.last_mut() {
                            previous.gas_cost += method_return.gas_cost;
                        }

                        // If the method called was from the target actor,
                        // we create a new call on `GasInfo` with the totals of gas charge.
                        let Some(method) = actor
                                .abi()
                                .methods()
                                .iter()
                                .find(|a| a.number() == method_return.num && method_return.at == *actor_id) else {
                            continue;
                        };

                        let mut gas_info = match info.methods.remove(method) {
                            Some(gi) => gi,
                            None => vec![],
                        };
                        gas_info.push(method_return.gas_cost);
                        info.methods.insert(method.clone(), gas_info);
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
                let min = calls.first().copied().unwrap_or_default();
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

#[cfg(test)]
mod tests {
    use kythera_lib::{
        Address, ApplyRet, ErrorNumber, ExitCode, Gas, GasCharge, RawBytes, Receipt, TokenAmount,
        WasmActor, Zero,
    };

    use super::*;

    const TARGET_ACTOR_ADDRESS: u64 = 44;

    #[test]
    fn analyzes_gas_consumption() {
        let m1 = Method::new_from_name("Method1").unwrap();
        let m1_number = m1.number();
        let m2 = Method::new_from_name("Method2").unwrap();
        let m2_number = m2.number();
        let target = WasmActor::new(
            "Target".into(),
            vec![],
            kythera_lib::Abi {
                constructor: None,
                set_up: None,
                methods: vec![m1, m2],
            },
        )
        .deploy(Address::new_id(44));
        let mut gr = GasReport::default();
        let result = TestResult::new(
            Method::new_from_name("TestMethod").unwrap(),
            TestResultType::Passed(ApplyRet {
                msg_receipt: Receipt {
                    exit_code: ExitCode::new(0),
                    return_data: RawBytes::default(),
                    gas_used: 0,
                    events_root: None,
                },
                penalty: TokenAmount::zero(),
                miner_tip: TokenAmount::zero(),
                base_fee_burn: TokenAmount::zero(),
                over_estimation_burn: TokenAmount::zero(),
                refund: TokenAmount::zero(),
                gas_refund: 0,
                gas_burned: 0,
                failure_info: None,
                exec_trace: vec![
                    ExecutionEvent::Call {
                        from: 0,
                        to: Address::new_id(TARGET_ACTOR_ADDRESS),
                        method: m1_number,
                        params: None,
                        value: TokenAmount::zero(),
                    },
                    ExecutionEvent::Call {
                        from: 0,
                        to: Address::new_id(TARGET_ACTOR_ADDRESS),
                        method: m2_number,
                        params: None,
                        value: TokenAmount::zero(),
                    },
                    ExecutionEvent::GasCharge(GasCharge::new(
                        "",
                        Gas::from_milligas(10),
                        Gas::from_milligas(20),
                    )),
                    ExecutionEvent::CallError(kythera_lib::SyscallError(
                        "error".into(),
                        ErrorNumber::Forbidden,
                    )),
                    ExecutionEvent::GasCharge(GasCharge::new(
                        "",
                        Gas::from_milligas(20),
                        Gas::from_milligas(30),
                    )),
                    ExecutionEvent::CallReturn(ExitCode::new(0), None),
                ],
                events: vec![],
            }),
        );
        gr.analyze(target.clone(), &[result]);
        let report = gr.reports.get(&target).unwrap();
        assert_eq!(report.methods.len(), 2);
        let m1m = report.methods.get(&m1_number).unwrap();
        assert_eq!(m1m.len(), 1);
        assert_eq!(m1m[0], 30);
        let m2m = report.methods.get(&m2_number).unwrap();
        assert_eq!(m2m.len(), 1);
        assert_eq!(m2m[0], 10);
    }
}
