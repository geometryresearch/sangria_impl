// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the Jellyfish library.

// You should have received a copy of the MIT License
// along with the Jellyfish library. If not, see <https://mit-license.org/>.

//! Polynomial Commitment Scheme
pub mod errors;
mod multilinear_kzg;
pub mod prelude;
mod structs;
mod transcript;
mod univariate_ipa;
mod univariate_kzg;

use core::ops::MulAssign;

use ark_ec::{AffineCurve, PairingEngine, ProjectiveCurve};
use ark_ff::{Field, PrimeField, SquareRootField};
use ark_poly::univariate::DensePolynomial;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{
    borrow::Borrow,
    fmt::Debug,
    hash::Hash,
    rand::{CryptoRng, RngCore},
    vec::Vec,
};
use errors::PCSError;

use crate::scalars_n_bases::ScalarsAndBases;

use self::prelude::Commitment;

/// A more restricted variant of the `PolynomialCommitmentScheme` trait
// TODO(fga): this should be simplified by https://github.com/rust-lang/rust/issues/41517
// type UVPCS<E> = PolynomialCommitmentScheme<E, Polynomial = DensePolynomial<E::Fr>, Commitment = Commitment<E>>;
pub trait UVPCS<E: CommitmentGroup>:
    PolynomialCommitmentScheme<
        E,
        Polynomial = DensePolynomial<<E as CommitmentGroup>::Fr>,
        Point = E::Fr,
        Evaluation = E::Fr,
        Commitment = Commitment<E>,
        BatchCommitment = Vec<Commitment<E>>,
        BatchProof = Vec<<Self as PolynomialCommitmentScheme<E>>::Proof>,
    > + Sync
{
    /// How to convert a commitment to the univariate PCS's proof
    fn from_comm(c: Commitment<E>) -> Self::Proof;
}

impl<
        E: CommitmentGroup,
        S: PolynomialCommitmentScheme<
                E,
                Point = E::Fr,
                Evaluation = E::Fr,
                Polynomial = DensePolynomial<<E as CommitmentGroup>::Fr>,
                Commitment = Commitment<E>,
                BatchCommitment = Vec<Commitment<E>>,
                BatchProof = Vec<<Self as PolynomialCommitmentScheme<E>>::Proof>,
            > + Sync,
    > UVPCS<E> for S
where
    S::Proof: From<Commitment<E>>,
{
    fn from_comm(c: Commitment<E>) -> Self::Proof {
        c.into()
    }
}

/// This trait defines the common APIs for a commitment group.
pub trait CommitmentGroup: Sized + 'static + Copy + Debug + Sync + Send + Eq + PartialEq {
    /// This is the scalar field of the group.
    type Fr: PrimeField + SquareRootField;

    /// The projective representation of an element in G1.
    type G1Projective: ProjectiveCurve<BaseField = Self::Fq, ScalarField = Self::Fr, Affine = Self::G1Affine>
        + From<Self::G1Affine>
        + Into<Self::G1Affine>
        + MulAssign<Self::Fr>; // needed due to https://github.com/rust-lang/rust/issues/69640

    /// The affine representation of an element in G1.
    type G1Affine: AffineCurve<BaseField = Self::Fq, ScalarField = Self::Fr, Projective = Self::G1Projective>
        + From<Self::G1Projective>
        + Into<Self::G1Projective>;

    /// The base field
    type Fq: PrimeField + SquareRootField;
}

impl<E: PairingEngine> CommitmentGroup for E {
    type Fr = E::Fr;
    type G1Affine = E::G1Affine;
    type G1Projective = E::G1Projective;
    type Fq = E::Fq;
}

/// This trait defines the max degree supported by an SRS
pub trait WithMaxDegree {
    /// Returns the max degree supported by the SRS
    fn max_degree(&self) -> usize;
}
/// This trait defines APIs for polynomial commitment schemes.
/// Note that for our usage, this PCS is not hiding.
/// TODO(#187): add hiding property.
pub trait PolynomialCommitmentScheme<E: CommitmentGroup> {
    /// Prover parameters
    type ProverParam: Clone
        + Debug
        + CanonicalSerialize
        + CanonicalDeserialize
        + PartialEq
        + Eq
        + Sync;
    /// Verifier parameters
    type VerifierParam: Clone
        + Debug
        + CanonicalSerialize
        + CanonicalDeserialize
        + PartialEq
        + Eq
        + Sync;
    /// Structured reference string
    type SRS: Clone + Debug + WithMaxDegree;
    /// Polynomial and its associated types
    type Polynomial: Clone
        + Debug
        + Hash
        + PartialEq
        + Eq
        + CanonicalSerialize
        + CanonicalDeserialize;
    /// Polynomial input domain
    type Point: Clone + Ord + Debug + Sync + Hash + PartialEq + Eq;
    /// Polynomial Evaluation
    type Evaluation: Field;
    /// Commitments
    type Commitment: Clone + CanonicalSerialize + CanonicalDeserialize + Debug + PartialEq + Eq;
    /// Batch commitments
    type BatchCommitment: Clone + CanonicalSerialize + CanonicalDeserialize + Debug + PartialEq + Eq;
    /// Proofs
    type Proof: Clone + CanonicalSerialize + CanonicalDeserialize + Debug + PartialEq + Eq;
    /// Batch proofs
    type BatchProof: Clone + CanonicalSerialize + CanonicalDeserialize + Debug + PartialEq + Eq;

    /// Build SRS for testing.
    ///
    /// - For univariate polynomials, `supported_size` is the maximum degree.
    /// - For multilinear polynomials, `supported_size` is the number of
    ///   variables.
    ///
    /// WARNING: THIS FUNCTION IS FOR TESTING PURPOSE ONLY.
    /// THE OUTPUT SRS SHOULD NOT BE USED IN PRODUCTION.
    fn gen_srs_for_testing<R: RngCore + CryptoRng>(
        rng: &mut R,
        supported_size: usize,
    ) -> Result<Self::SRS, PCSError>;

    /// Trim the universal parameters to specialize the public parameters.
    /// Input both `supported_degree` for univariate and
    /// `supported_num_vars` for multilinear.
    /// ## Note on function signature
    /// Usually, data structure like SRS and ProverParam are huge and users
    /// might wish to keep them in heap using different kinds of smart pointers
    /// (instead of only in stack) therefore our `impl Borrow<_>` interface
    /// allows for passing in any pointer type, e.g.: `trim(srs: &Self::SRS,
    /// ..)` or `trim(srs: Box<Self::SRS>, ..)` or `trim(srs: Arc<Self::SRS>,
    /// ..)` etc.
    fn trim(
        srs: impl Borrow<Self::SRS>,
        supported_degree: usize,
        supported_num_vars: Option<usize>,
    ) -> Result<(Self::ProverParam, Self::VerifierParam), PCSError>;

    /// Generate a commitment for a polynomial
    /// ## Note on function signature
    /// Usually, data structure like SRS and ProverParam are huge and users
    /// might wish to keep them in heap using different kinds of smart pointers
    /// (instead of only in stack) therefore our `impl Borrow<_>` interface
    /// allows for passing in any pointer type, e.g.: `commit(prover_param:
    /// &Self::ProverParam, ..)` or `commit(prover_param:
    /// Box<Self::ProverParam>, ..)` or `commit(prover_param:
    /// Arc<Self::ProverParam>, ..)` etc.
    /// Also, the commitment is not hiding.
    fn commit(
        prover_param: impl Borrow<Self::ProverParam>,
        poly: &Self::Polynomial,
    ) -> Result<Self::Commitment, PCSError>;

    /// Batch commit a list of polynomials
    fn batch_commit(
        prover_param: impl Borrow<Self::ProverParam>,
        polys: &[Self::Polynomial],
    ) -> Result<Self::BatchCommitment, PCSError>;

    /// On input a polynomial `p` and a point `point`, outputs a proof for the
    /// same.
    fn open(
        prover_param: impl Borrow<Self::ProverParam>,
        polynomial: &Self::Polynomial,
        point: &Self::Point,
    ) -> Result<(Self::Proof, Self::Evaluation), PCSError>;

    /// Input a list of polynomials, and a same number of points,
    /// compute a batch opening for all the polynomials.
    fn batch_open(
        prover_param: impl Borrow<Self::ProverParam>,
        batch_commitment: &Self::BatchCommitment,
        polynomials: &[Self::Polynomial],
        points: &[Self::Point],
    ) -> Result<(Self::BatchProof, Vec<Self::Evaluation>), PCSError>;

    /// Verifies that `value` is the evaluation at `x` of the polynomial
    /// committed inside `comm`.
    fn verify(
        verifier_param: &Self::VerifierParam,
        commitment: &Self::Commitment,
        point: &Self::Point,
        value: &E::Fr,
        proof: &Self::Proof,
    ) -> Result<bool, PCSError>;

    /// Verifies that `value_i` is the evaluation at `x_i` of the polynomial
    /// `poly_i` committed inside `comm`.
    fn batch_verify<I: IntoIterator<Item = E::Fr>>(
        verifier_param: &Self::VerifierParam,
        multi_commitment: &Self::BatchCommitment,
        points: &[Self::Point],
        values: &[E::Fr],
        batch_proof: &Self::BatchProof,
        randomizers: I,
    ) -> Result<bool, PCSError>;

    /// Verifies that a pipelined set of batch proofs is valid.
    /// A "pipelined" set of batch proofs is a set of batch proof expressed in the form of a
    /// sequence of batch proofs.
    fn batch_verify_aggregated<I: IntoIterator<Item = E::Fr>, const ARITY: usize>(
        verifier_param: &Self::VerifierParam,
        multi_commitment: &[ScalarsAndBases<E>],
        points: [&[Self::Point]; ARITY],
        values: &[E::Fr],
        batch_proof: [&Self::BatchProof; ARITY],
        combiners: [&[E::Fr]; ARITY], // the combiners for the linear combination of the batch proofs
        randomizers: I,
    ) -> Result<bool, PCSError>;
}

/// API definitions for structured reference string
pub trait StructuredReferenceString<E: PairingEngine>: Sized {
    /// Prover parameters
    type ProverParam;
    /// Verifier parameters
    type VerifierParam;

    /// Extract the prover parameters from the public parameters.
    fn extract_prover_param(&self, supported_size: usize) -> Self::ProverParam;
    /// Extract the verifier parameters from the public parameters.
    fn extract_verifier_param(&self, supported_size: usize) -> Self::VerifierParam;

    /// Trim the universal parameters to specialize the public parameters
    /// for polynomials to the given `supported_size`, and
    /// returns committer key and verifier key.
    ///
    /// - For univariate polynomials, `supported_size` is the maximum degree.
    /// - For multilinear polynomials, `supported_size` is 2 to the number of
    ///   variables.
    ///
    /// `supported_log_size` should be in range `1..=params.log_size`
    fn trim(
        &self,
        supported_size: usize,
    ) -> Result<(Self::ProverParam, Self::VerifierParam), PCSError>;

    /// Build SRS for testing.
    ///
    /// - For univariate polynomials, `supported_size` is the maximum degree.
    /// - For multilinear polynomials, `supported_size` is the number of
    ///   variables.
    ///
    /// WARNING: THIS FUNCTION IS FOR TESTING PURPOSE ONLY.
    /// THE OUTPUT SRS SHOULD NOT BE USED IN PRODUCTION.
    fn gen_srs_for_testing<R: RngCore + CryptoRng>(
        rng: &mut R,
        supported_size: usize,
    ) -> Result<Self, PCSError>;
}
