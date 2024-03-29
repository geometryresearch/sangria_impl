#![warn(missing_docs)]
//! Sangria is a Nova-like scheme for recursive zero-knowledge proofs. It implements incrementally
//! verifiable computation by using a folding for PLONK. We use a modified version of HyperPlonk to
//! compress the IVC proofs.

use ark_ff::PrimeField;
use ark_std::rand::Rng;

/// Interface for an IVC scheme.
pub trait IVC<F: PrimeField, SC: StepCircuit<F>> {
    /// Public parameters for the IVC scheme.
    type PublicParameters;

    /// A collection of data needed for proving.
    type ProverKey;

    /// A collection of data needed for verifying.
    type VerifierKey;

    /// An IVC proof.
    type Proof;

    /// Run the IVC setup to produce public parameters.
    fn setup<R: Rng>(rng: &mut R) -> Self::PublicParameters;

    /// Run the IVC encoder to produce a proving key and a verifying key.
    fn encode<R: Rng>(
        public_parameters: &Self::PublicParameters,
        step_circuit: &SC,
        rng: &mut R,
    ) -> Result<(Self::ProverKey, Self::VerifierKey), SangriaError>;

    /// Prove a step of the IVC computation. Consume the current state and proof and produce the *next* state and proof.
    fn prove_step(
        prover_key: &Self::ProverKey,
        origin_state: &SC::State,
        current_state: SC::State,
        current_proof: Option<Self::Proof>,
        current_witness: &SC::Witness,
    ) -> Result<(SC::State, Self::Proof), SangriaError>;

    /// Verify a step of the IVC computation.
    fn verify(
        verifier_key: &Self::VerifierKey,
        origin_state: &SC::State,
        current_state: SC::State,
        current_proof: Option<Self::Proof>,
    ) -> Result<(), SangriaError>;
}

/// A marker trait for an IVC scheme which implements proof compression.
pub trait IVCWithProofCompression<F: PrimeField, SC: StepCircuit<F>>: IVC<F, SC> {}

/// Interface for a single step of the incremental computation.
pub trait StepCircuit<F: PrimeField> {
    /// The output a single step of the IVC.
    type State;

    /// The non-deterministic input for a step of the computation
    type Witness;
}

/// Interface for a non-interactive folding scheme (NIFS).
pub trait NonInteractiveFoldingScheme {
    /// A type to contain the arguments necessary to run `setup`
    type SetupInfo;

    /// Public parameters for the scheme.
    type PublicParameters;

    /// The structure of the underlying NP problem.
    type Structure;

    /// A collection of data needed for proving.
    type ProverKey;

    /// A collection of data needed for verifying.
    type VerifierKey;

    /// An instance of the relation that will be folded.
    type Instance;

    /// A witness for the relation to be folded.
    type Witness;

    /// The prover's message.
    type ProverMessage;

    /// Run the randomised setup for the folding scheme to produce public parameters.
    fn setup<R: Rng>(info: &Self::SetupInfo, rng: &mut R) -> Self::PublicParameters;

    /// Using the public parameters, run the randomised encoder that produces a prover key and verifier key.
    fn encode<R: Rng>(
        pp: &Self::PublicParameters,
        circuit: &Self::Structure,
        rng: &mut R,
    ) -> Result<(Self::ProverKey, Self::VerifierKey), SangriaError>;

    /// The folding scheme prover. Outputs a folded instance-witness pair and the prover's message.
    #[allow(clippy::type_complexity)]
    fn prover(
        public_parameters: &Self::PublicParameters,
        prover_key: &Self::ProverKey,
        left_instance: &Self::Instance,
        left_witness: &Self::Witness,
        right_instance: &Self::Instance,
        right_witness: &Self::Witness,
    ) -> Result<(Self::Instance, Self::Witness, Self::ProverMessage), SangriaError>;

    /// The folding scheme verifier. Outputs a folded instance.
    fn verifier(
        public_parameters: &Self::PublicParameters,
        verifier_key: &Self::VerifierKey,
        left_instance: &Self::Instance,
        right_instance: &Self::Instance,
        prover_message: &Self::ProverMessage,
    ) -> Result<Self::Instance, SangriaError>;
}

mod folding_scheme;
pub use folding_scheme::PLONKFoldingScheme;

// mod ivc;

mod relaxed_plonk;
pub use relaxed_plonk::{
    PLONKCircuit, RelaxedPLONKInstance, RelaxedPLONKWitness, CONSTANT_SELECTOR_INDEX,
    LEFT_SELECTOR_INDEX, MULTIPLICATION_SELECTOR_INDEX, OUTPUT_SELECTOR_INDEX,
    RIGHT_SELECTOR_INDEX,
};

mod sangria;
pub use sangria::Sangria;

mod errors;
pub use errors::SangriaError;

mod vector_commitment;
