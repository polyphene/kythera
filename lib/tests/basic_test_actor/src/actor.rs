use frc42_dispatch::match_method;
use fvm_ipld_encoding::DAG_CBOR;
use fvm_sdk as sdk;
use fvm_shared::error::ExitCode;
use sdk::sys::ErrorNumber;
use serde::ser;
use thiserror::Error;

#[derive(Error, Debug)]
enum IpldError {
    #[error("ipld encoding error: {0}")]
    Encoding(#[from] fvm_ipld_encoding::Error),
    #[error("ipld blockstore error: {0}")]
    Blockstore(#[from] ErrorNumber),
}

fn return_ipld<T>(value: &T) -> std::result::Result<u32, IpldError>
where
    T: ser::Serialize + ?Sized,
{
    let bytes = fvm_ipld_encoding::to_vec(value)?;
    Ok(sdk::ipld::put_block(DAG_CBOR, bytes.as_slice())?)
}

#[no_mangle]
fn invoke(_input: u32) -> u32 {
    std::panic::set_hook(Box::new(|info| {
        sdk::vm::abort(
            ExitCode::USR_ASSERTION_FAILED.value(),
            Some(&format!("{info}")),
        )
    }));

    let method_num = sdk::message::method_number();
    match_method!(method_num, {
        "TestOne" => {
            return_ipld("TestOne").unwrap()
        },
        "TestTwo" => {
            return_ipld("TestTwo").unwrap()
        },
        _ => {
            sdk::vm::abort(
                ExitCode::USR_UNHANDLED_MESSAGE.value(),
                Some("Unknown method number"),
            );
        }
    })
}
