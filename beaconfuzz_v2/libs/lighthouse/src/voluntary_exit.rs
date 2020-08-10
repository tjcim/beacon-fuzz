use state_processing::{
    per_block_processing::{process_exits, VerifySignatures},
    BlockProcessingError,
};

use types::{BeaconState, EthSpec, MainnetEthSpec, SignedVoluntaryExit};

pub fn process_voluntary_exit(
    mut beaconstate: BeaconState<MainnetEthSpec>,
    voluntary_exit: SignedVoluntaryExit,
) -> Result<BeaconState<MainnetEthSpec>, BlockProcessingError> {
    let spec = MainnetEthSpec::default_spec();

    process_exits(
        &mut beaconstate,
        &[voluntary_exit],
        VerifySignatures::False,
        &spec,
    )?;

    Ok(beaconstate)
}