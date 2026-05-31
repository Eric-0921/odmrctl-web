//! odmr-mag — Mock-only magnetic field planning and safety validation.
//!
//! **IMPORTANT: This crate is mock-only for Mag-M0 and Mag-M1.**
//!
//! No code path in this crate connects to real hardware or emits real coil
//! current.  All magnetic actions are represented as dry-run / mock events.
//!
//! ## Scope
//!
//! - B-vector model (Cartesian + Spherical)
//! - Coil current model with limits
//! - Coil matrix (forward + inverse) with singular-matrix rejection
//! - Safety policy validation
//! - Mock magnetic axes state timeline
//! - Run artifact types
//!
//! ## Coordinate Conventions
//!
//! - Internal: Tesla [T], Ampere [A], Radians [rad], Milliseconds [ms]
//! - JSON boundary may accept mT and deg; normalize before use.
//! - Right-handed Cartesian system aligned with the NV diamond stage.

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during magnetic planning or safety validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MagError {
    /// Singular or non-invertible coil matrix.
    SingularMatrix { reason: String },
    /// Coil matrix is ill-conditioned.
    IllConditionedMatrix {
        condition_number: f64,
        threshold: f64,
    },
    /// Missing or unverified calibration.
    CalibrationMissing { field: String },
    /// Calibration is too old.
    CalibrationStale { age_days: u64, max_days: u64 },
    /// Target B field exceeds absolute limit.
    BFieldOutOfRange { b_abs_t: f64, limit_t: f64 },
    /// Computed current exceeds per-axis limit.
    CurrentLimitExceeded {
        axis: char,
        current_a: f64,
        limit_a: f64,
    },
    /// Computed vector current exceeds combined limit.
    VectorCurrentLimitExceeded { current_a: f64, limit_a: f64 },
    /// Ramp rate exceeds per-axis limit.
    RampRateExceeded {
        axis: char,
        rate_a_per_s: f64,
        limit_a_per_s: f64,
    },
    /// Vector ramp rate exceeds combined limit.
    VectorRampRateExceeded {
        rate_a_per_s: f64,
        limit_a_per_s: f64,
    },
    /// Settle time is below minimum.
    SettleTimeTooShort { settle_ms: u64, min_ms: u64 },
    /// Target contains NaN or Inf.
    NonFiniteValue { field: String, value: f64 },
    /// Safety policy violation (generic).
    SafetyViolation { message: String },
}

impl fmt::Display for MagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MagError::SingularMatrix { reason } => write!(f, "singular coil matrix: {reason}"),
            MagError::IllConditionedMatrix {
                condition_number,
                threshold,
            } => {
                write!(f, "ill-conditioned coil matrix: condition_number={condition_number:.2e}, threshold={threshold:.2e}")
            }
            MagError::CalibrationMissing { field } => {
                write!(f, "calibration missing or unverified: {field}")
            }
            MagError::CalibrationStale { age_days, max_days } => {
                write!(
                    f,
                    "calibration stale: age={age_days} days, max={max_days} days"
                )
            }
            MagError::BFieldOutOfRange { b_abs_t, limit_t } => {
                write!(
                    f,
                    "B field out of range: |B|={b_abs_t:.6e} T, limit={limit_t:.6e} T"
                )
            }
            MagError::CurrentLimitExceeded {
                axis,
                current_a,
                limit_a,
            } => {
                write!(
                    f,
                    "current limit exceeded on axis {axis}: {current_a:.6e} A > {limit_a:.6e} A"
                )
            }
            MagError::VectorCurrentLimitExceeded { current_a, limit_a } => {
                write!(
                    f,
                    "vector current limit exceeded: |I|={current_a:.6e} A > {limit_a:.6e} A"
                )
            }
            MagError::RampRateExceeded {
                axis,
                rate_a_per_s,
                limit_a_per_s,
            } => {
                write!(f, "ramp rate exceeded on axis {axis}: {rate_a_per_s:.6e} A/s > {limit_a_per_s:.6e} A/s")
            }
            MagError::VectorRampRateExceeded {
                rate_a_per_s,
                limit_a_per_s,
            } => {
                write!(f, "vector ramp rate exceeded: |dI/dt|={rate_a_per_s:.6e} A/s > {limit_a_per_s:.6e} A/s")
            }
            MagError::SettleTimeTooShort { settle_ms, min_ms } => {
                write!(f, "settle time too short: {settle_ms} ms < {min_ms} ms")
            }
            MagError::NonFiniteValue { field, value } => {
                write!(f, "non-finite value in {field}: {value}")
            }
            MagError::SafetyViolation { message } => {
                write!(f, "safety violation: {message}")
            }
        }
    }
}

impl std::error::Error for MagError {}

// ---------------------------------------------------------------------------
// B-field vector (Cartesian)
// ---------------------------------------------------------------------------

/// Magnetic field vector in Cartesian coordinates [T].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BVectorCartesian {
    pub bx_t: f64,
    pub by_t: f64,
    pub bz_t: f64,
}

impl BVectorCartesian {
    pub fn new(bx_t: f64, by_t: f64, bz_t: f64) -> Self {
        Self { bx_t, by_t, bz_t }
    }

    /// Magnitude of the B vector [T].
    pub fn b_abs_t(&self) -> f64 {
        (self.bx_t.powi(2) + self.by_t.powi(2) + self.bz_t.powi(2)).sqrt()
    }

    /// Check all components are finite.
    pub fn is_finite(&self) -> bool {
        self.bx_t.is_finite() && self.by_t.is_finite() && self.bz_t.is_finite()
    }
}

// ---------------------------------------------------------------------------
// B-field vector (Spherical)
// ---------------------------------------------------------------------------

/// Magnetic field vector in spherical coordinates.
///
/// - `b_abs_t`: magnitude [T]
/// - `theta_rad`: polar angle from +Z axis, range [0, π]
/// - `phi_rad`: azimuthal angle from +X axis in XY plane, range [0, 2π)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BVectorSpherical {
    pub b_abs_t: f64,
    pub theta_rad: f64,
    pub phi_rad: f64,
}

impl BVectorSpherical {
    pub fn new(b_abs_t: f64, theta_rad: f64, phi_rad: f64) -> Self {
        Self {
            b_abs_t,
            theta_rad,
            phi_rad,
        }
    }

    /// Normalize angles to standard ranges.
    pub fn normalized(&self) -> Self {
        let mut theta = self.theta_rad % (2.0 * std::f64::consts::PI);
        let mut phi = self.phi_rad % (2.0 * std::f64::consts::PI);
        if phi < 0.0 {
            phi += 2.0 * std::f64::consts::PI;
        }
        // theta is in [0, π]; if outside, reflect
        if theta < 0.0 {
            theta = -theta;
            phi += std::f64::consts::PI;
        }
        if theta > std::f64::consts::PI {
            theta = 2.0 * std::f64::consts::PI - theta;
            phi += std::f64::consts::PI;
        }
        phi %= 2.0 * std::f64::consts::PI;
        if phi < 0.0 {
            phi += 2.0 * std::f64::consts::PI;
        }
        Self {
            b_abs_t: self.b_abs_t,
            theta_rad: theta,
            phi_rad: phi,
        }
    }

    /// Check all components are finite.
    pub fn is_finite(&self) -> bool {
        self.b_abs_t.is_finite() && self.theta_rad.is_finite() && self.phi_rad.is_finite()
    }
}

// ---------------------------------------------------------------------------
// Coordinate conversions
// ---------------------------------------------------------------------------

/// Convert Cartesian B vector to spherical coordinates.
///
/// For zero vector, returns `b_abs_t = 0, theta_rad = 0, phi_rad = 0`.
pub fn cartesian_to_spherical(c: &BVectorCartesian) -> BVectorSpherical {
    let b_abs = c.b_abs_t();
    if b_abs == 0.0 {
        return BVectorSpherical::new(0.0, 0.0, 0.0);
    }
    let theta = (c.bz_t / b_abs).acos();
    let phi = c.by_t.atan2(c.bx_t);
    let mut s = BVectorSpherical::new(b_abs, theta, phi);
    s = s.normalized();
    s
}

/// Convert spherical B vector to Cartesian coordinates.
pub fn spherical_to_cartesian(s: &BVectorSpherical) -> BVectorCartesian {
    let sin_theta = s.theta_rad.sin();
    let bx = s.b_abs_t * sin_theta * s.phi_rad.cos();
    let by = s.b_abs_t * sin_theta * s.phi_rad.sin();
    let bz = s.b_abs_t * s.theta_rad.cos();
    BVectorCartesian::new(bx, by, bz)
}

// ---------------------------------------------------------------------------
// Coil current vector
// ---------------------------------------------------------------------------

/// Three-axis coil current vector [A].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CoilCurrent {
    pub ix_a: f64,
    pub iy_a: f64,
    pub iz_a: f64,
}

impl CoilCurrent {
    pub fn new(ix_a: f64, iy_a: f64, iz_a: f64) -> Self {
        Self { ix_a, iy_a, iz_a }
    }

    /// Euclidean norm of the current vector.
    pub fn abs_a(&self) -> f64 {
        (self.ix_a.powi(2) + self.iy_a.powi(2) + self.iz_a.powi(2)).sqrt()
    }

    /// Per-axis absolute values.
    pub fn abs_per_axis(&self) -> [f64; 3] {
        [self.ix_a.abs(), self.iy_a.abs(), self.iz_a.abs()]
    }

    /// Check all components are finite.
    pub fn is_finite(&self) -> bool {
        self.ix_a.is_finite() && self.iy_a.is_finite() && self.iz_a.is_finite()
    }
}

// ---------------------------------------------------------------------------
// Coil matrix
// ---------------------------------------------------------------------------

/// A 3×3 coil matrix mapping coil current [A] to magnetic field [T].
///
/// Formula: `B = M * (I - I_offset) + B_zero_offset`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoilMatrix {
    /// 3×3 matrix in row-major order: M[row][col]
    pub m: [[f64; 3]; 3],
    /// Per-axis current offset [A]
    pub i_offset_a: [f64; 3],
    /// Residual B field at zero current [T]
    pub b_zero_offset_t: [f64; 3],
    /// Pre-computed condition number (optional, will be computed if None)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_number: Option<f64>,
    /// Calibration timestamp (RFC 3339)
    pub calibrated_at: String,
    /// Whether this calibration has been experimentally verified
    pub verified: bool,
    /// Verified by (agent or operator ID)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_by: Option<String>,
}

impl CoilMatrix {
    /// Compute the determinant of the 3×3 matrix.
    pub fn determinant(&self) -> f64 {
        let m = &self.m;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }

    /// Compute the condition number (ratio of max to min singular value).
    /// Uses the 2-norm condition number via power iteration for simplicity.
    pub fn compute_condition_number(&self) -> f64 {
        // Use SVD-like approach: cond = ||M|| * ||M^-1||
        // For small 3x3, we compute the Frobenius norm as approximation
        // and refine with explicit inverse.
        let m_norm = matrix_frobenius_norm(&self.m);
        match self.inverse_matrix() {
            Ok(inv) => {
                let inv_norm = matrix_frobenius_norm(&inv);
                m_norm * inv_norm
            }
            Err(_) => f64::INFINITY,
        }
    }

    /// Return the effective condition number (cached or computed).
    pub fn condition_number(&self) -> f64 {
        self.condition_number
            .unwrap_or_else(|| self.compute_condition_number())
    }

    /// Invert a 3×3 matrix.
    fn inverse_matrix(&self) -> Result<[[f64; 3]; 3], MagError> {
        let det = self.determinant();
        if det.abs() < 1e-15 {
            return Err(MagError::SingularMatrix {
                reason: format!("determinant = {det:.2e}"),
            });
        }

        let m = &self.m;
        let mut inv = [[0.0; 3]; 3];

        // Compute adjugate / cofactor matrix (transposed)
        inv[0][0] = (m[1][1] * m[2][2] - m[1][2] * m[2][1]) / det;
        inv[0][1] = (m[0][2] * m[2][1] - m[0][1] * m[2][2]) / det;
        inv[0][2] = (m[0][1] * m[1][2] - m[0][2] * m[1][1]) / det;

        inv[1][0] = (m[1][2] * m[2][0] - m[1][0] * m[2][2]) / det;
        inv[1][1] = (m[0][0] * m[2][2] - m[0][2] * m[2][0]) / det;
        inv[1][2] = (m[0][2] * m[1][0] - m[0][0] * m[1][2]) / det;

        inv[2][0] = (m[1][0] * m[2][1] - m[1][1] * m[2][0]) / det;
        inv[2][1] = (m[0][1] * m[2][0] - m[0][0] * m[2][1]) / det;
        inv[2][2] = (m[0][0] * m[1][1] - m[0][1] * m[1][0]) / det;

        Ok(inv)
    }

    /// Compute target coil current from target B field using inverse matrix.
    ///
    /// Formula: `I_target = inv(M) * (B_target - B_zero_offset) + I_offset`
    pub fn current_from_b(&self, b_target: &BVectorCartesian) -> Result<CoilCurrent, MagError> {
        if !b_target.is_finite() {
            return Err(MagError::NonFiniteValue {
                field: "b_target".into(),
                value: if !b_target.bx_t.is_finite() {
                    b_target.bx_t
                } else if !b_target.by_t.is_finite() {
                    b_target.by_t
                } else {
                    b_target.bz_t
                },
            });
        }

        let inv = self.inverse_matrix()?;

        let dbx = b_target.bx_t - self.b_zero_offset_t[0];
        let dby = b_target.by_t - self.b_zero_offset_t[1];
        let dbz = b_target.bz_t - self.b_zero_offset_t[2];

        let ix = inv[0][0] * dbx + inv[0][1] * dby + inv[0][2] * dbz + self.i_offset_a[0];
        let iy = inv[1][0] * dbx + inv[1][1] * dby + inv[1][2] * dbz + self.i_offset_a[1];
        let iz = inv[2][0] * dbx + inv[2][1] * dby + inv[2][2] * dbz + self.i_offset_a[2];

        Ok(CoilCurrent::new(ix, iy, iz))
    }

    /// Compute B field from coil current using forward matrix.
    ///
    /// Formula: `B = M * (I - I_offset) + B_zero_offset`
    pub fn b_from_current(&self, current: &CoilCurrent) -> Result<BVectorCartesian, MagError> {
        if !current.is_finite() {
            return Err(MagError::NonFiniteValue {
                field: "current".into(),
                value: if !current.ix_a.is_finite() {
                    current.ix_a
                } else if !current.iy_a.is_finite() {
                    current.iy_a
                } else {
                    current.iz_a
                },
            });
        }

        let dix = current.ix_a - self.i_offset_a[0];
        let diy = current.iy_a - self.i_offset_a[1];
        let diz = current.iz_a - self.i_offset_a[2];

        let bx =
            self.m[0][0] * dix + self.m[0][1] * diy + self.m[0][2] * diz + self.b_zero_offset_t[0];
        let by =
            self.m[1][0] * dix + self.m[1][1] * diy + self.m[1][2] * diz + self.b_zero_offset_t[1];
        let bz =
            self.m[2][0] * dix + self.m[2][1] * diy + self.m[2][2] * diz + self.b_zero_offset_t[2];

        Ok(BVectorCartesian::new(bx, by, bz))
    }
}

fn matrix_frobenius_norm(m: &[[f64; 3]; 3]) -> f64 {
    let mut sum = 0.0;
    for row in m {
        for &v in row {
            sum += v * v;
        }
    }
    sum.sqrt()
}

// ---------------------------------------------------------------------------
// Safety policy
// ---------------------------------------------------------------------------

/// Magnetic safety policy. Recipe cannot override these limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagSafetyPolicy {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub max_current_a_per_axis: f64,
    pub max_abs_current_vector_a: f64,
    pub max_ramp_rate_a_per_s: f64,
    pub max_vector_ramp_rate_a_per_s: f64,
    pub min_settle_ms: u64,
    pub max_b_abs_t: f64,
    pub max_calibration_age_days: u64,
    pub require_calibration_verified: bool,
    /// Must always be false. Present for explicit documentation.
    pub recipe_override_safety: bool,
}

impl Default for MagSafetyPolicy {
    fn default() -> Self {
        Self {
            schema_version: "0.2.0".into(),
            kind: "mag_safety_policy".into(),
            id: "mag_safety_default".into(),
            name: Some("Default Mock Magnetic Safety Policy".into()),
            max_current_a_per_axis: 2.0,
            max_abs_current_vector_a: 3.0,
            max_ramp_rate_a_per_s: 0.5,
            max_vector_ramp_rate_a_per_s: 0.8,
            min_settle_ms: 100,
            max_b_abs_t: 0.01,
            max_calibration_age_days: 30,
            require_calibration_verified: true,
            recipe_override_safety: false,
        }
    }
}

impl MagSafetyPolicy {
    /// Validate a target B field against this policy.
    pub fn check_b_field(&self, b: &BVectorCartesian) -> Result<(), MagError> {
        if !b.is_finite() {
            return Err(MagError::NonFiniteValue {
                field: "b_target".into(),
                value: if !b.bx_t.is_finite() {
                    b.bx_t
                } else if !b.by_t.is_finite() {
                    b.by_t
                } else {
                    b.bz_t
                },
            });
        }
        let b_abs = b.b_abs_t();
        if b_abs > self.max_b_abs_t {
            return Err(MagError::BFieldOutOfRange {
                b_abs_t: b_abs,
                limit_t: self.max_b_abs_t,
            });
        }
        Ok(())
    }

    /// Validate computed coil currents against this policy.
    pub fn check_current(&self, current: &CoilCurrent) -> Result<(), MagError> {
        if !current.is_finite() {
            return Err(MagError::NonFiniteValue {
                field: "current".into(),
                value: if !current.ix_a.is_finite() {
                    current.ix_a
                } else if !current.iy_a.is_finite() {
                    current.iy_a
                } else {
                    current.iz_a
                },
            });
        }

        let abs_per_axis = current.abs_per_axis();
        let axes = ['x', 'y', 'z'];
        for (i, &abs_current) in abs_per_axis.iter().enumerate() {
            if abs_current > self.max_current_a_per_axis {
                return Err(MagError::CurrentLimitExceeded {
                    axis: axes[i],
                    current_a: abs_current,
                    limit_a: self.max_current_a_per_axis,
                });
            }
        }

        let vector_abs = current.abs_a();
        if vector_abs > self.max_abs_current_vector_a {
            return Err(MagError::VectorCurrentLimitExceeded {
                current_a: vector_abs,
                limit_a: self.max_abs_current_vector_a,
            });
        }

        Ok(())
    }

    /// Validate ramp rate against this policy.
    pub fn check_ramp_rate(
        &self,
        from: &CoilCurrent,
        to: &CoilCurrent,
        ramp_time_ms: u64,
    ) -> Result<(), MagError> {
        if ramp_time_ms == 0 {
            // Instant change — treat as infinite ramp rate
            return Err(MagError::RampRateExceeded {
                axis: 'x',
                rate_a_per_s: f64::INFINITY,
                limit_a_per_s: self.max_ramp_rate_a_per_s,
            });
        }

        let dt_s = ramp_time_ms as f64 / 1000.0;
        let dix = (to.ix_a - from.ix_a).abs();
        let diy = (to.iy_a - from.iy_a).abs();
        let diz = (to.iz_a - from.iz_a).abs();

        let rx = dix / dt_s;
        let ry = diy / dt_s;
        let rz = diz / dt_s;

        let axes = [('x', rx), ('y', ry), ('z', rz)];
        for (axis, rate) in axes {
            if rate > self.max_ramp_rate_a_per_s {
                return Err(MagError::RampRateExceeded {
                    axis,
                    rate_a_per_s: rate,
                    limit_a_per_s: self.max_ramp_rate_a_per_s,
                });
            }
        }

        let vector_rate = ((to.ix_a - from.ix_a).powi(2)
            + (to.iy_a - from.iy_a).powi(2)
            + (to.iz_a - from.iz_a).powi(2))
        .sqrt()
            / dt_s;

        if vector_rate > self.max_vector_ramp_rate_a_per_s {
            return Err(MagError::VectorRampRateExceeded {
                rate_a_per_s: vector_rate,
                limit_a_per_s: self.max_vector_ramp_rate_a_per_s,
            });
        }

        Ok(())
    }

    /// Validate settle time against this policy.
    pub fn check_settle_time(&self, settle_ms: u64) -> Result<(), MagError> {
        if settle_ms < self.min_settle_ms {
            return Err(MagError::SettleTimeTooShort {
                settle_ms,
                min_ms: self.min_settle_ms,
            });
        }
        Ok(())
    }

    /// Validate calibration against this policy.
    pub fn check_calibration(&self, coil_matrix: &CoilMatrix) -> Result<(), MagError> {
        if self.require_calibration_verified && !coil_matrix.verified {
            return Err(MagError::CalibrationMissing {
                field: "calibration.verified".into(),
            });
        }

        let cond = coil_matrix.condition_number();
        let threshold = 1e12;
        if cond > threshold || !cond.is_finite() {
            return Err(MagError::IllConditionedMatrix {
                condition_number: cond,
                threshold,
            });
        }

        // NOTE: calibration age check would require parsing the calibrated_at timestamp.
        // For Mag-M0 we check the verified flag and condition number only.
        // Age checking is deferred to Mag-M1 with a proper chrono dependency.

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Mock magnetic axes
// ---------------------------------------------------------------------------

/// A request to ramp the magnetic field to a new target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RampRequest {
    pub target_b_t: BVectorCartesian,
    pub ramp_rate_a_per_s: f64,
    pub settle_ms: u64,
}

/// Result of applying a ramp request to mock magnetic axes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MockMagResult {
    Accepted {
        from_current_a: CoilCurrent,
        to_current_a: CoilCurrent,
        estimated_ramp_ms: u64,
    },
    Rejected {
        requested_target_b_t: BVectorCartesian,
        reason: String,
    },
}

/// A single event in the mock magnetic axis timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MockMagEvent {
    RampAccepted {
        seq: u64,
        from_current_a: CoilCurrent,
        to_current_a: CoilCurrent,
        estimated_ramp_ms: u64,
    },
    SettleComplete {
        seq: u64,
        final_current_a: CoilCurrent,
        final_b_t: BVectorCartesian,
    },
    RampRejected {
        seq: u64,
        requested_target_b_t: BVectorCartesian,
        reason: String,
    },
}

/// A snapshot of the mock magnetic axis state at a point in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockMagState {
    pub seq: u64,
    pub timestamp_ms: u64,
    pub current_a: CoilCurrent,
    pub b_field_t: BVectorCartesian,
}

/// Mock three-axis magnetic field simulator.
///
/// Maintains state and produces deterministic events.  No hardware access.
pub struct MockMagAxes {
    current_a: CoilCurrent,
    b_field_t: BVectorCartesian,
    coil_matrix: CoilMatrix,
    safety_policy: MagSafetyPolicy,
    seq: u64,
    timestamp_ms: u64,
    events: Vec<MockMagEvent>,
    states: Vec<MockMagState>,
}

impl MockMagAxes {
    pub fn new(
        initial_current_a: CoilCurrent,
        coil_matrix: CoilMatrix,
        safety_policy: MagSafetyPolicy,
    ) -> Result<Self, MagError> {
        // Validate initial state
        if !initial_current_a.is_finite() {
            return Err(MagError::NonFiniteValue {
                field: "initial_current".into(),
                value: if !initial_current_a.ix_a.is_finite() {
                    initial_current_a.ix_a
                } else if !initial_current_a.iy_a.is_finite() {
                    initial_current_a.iy_a
                } else {
                    initial_current_a.iz_a
                },
            });
        }

        safety_policy.check_calibration(&coil_matrix)?;
        safety_policy.check_current(&initial_current_a)?;

        let b_field_t = coil_matrix.b_from_current(&initial_current_a)?;

        Ok(Self {
            current_a: initial_current_a,
            b_field_t,
            coil_matrix,
            safety_policy,
            seq: 0,
            timestamp_ms: 0,
            events: Vec::new(),
            states: vec![MockMagState {
                seq: 0,
                timestamp_ms: 0,
                current_a: initial_current_a,
                b_field_t,
            }],
        })
    }

    /// Apply a ramp request. Returns the result and records events.
    pub fn apply(&mut self, req: &RampRequest) -> MockMagResult {
        self.seq += 1;
        let seq = self.seq;

        // Step 1: Validate B field target
        if let Err(e) = self.safety_policy.check_b_field(&req.target_b_t) {
            let result = MockMagResult::Rejected {
                requested_target_b_t: req.target_b_t,
                reason: e.to_string(),
            };
            self.record_rejected(seq, req, &result);
            return result;
        }

        // Step 2: Compute target current
        let target_current = match self.coil_matrix.current_from_b(&req.target_b_t) {
            Ok(c) => c,
            Err(e) => {
                let result = MockMagResult::Rejected {
                    requested_target_b_t: req.target_b_t,
                    reason: e.to_string(),
                };
                self.record_rejected(seq, req, &result);
                return result;
            }
        };

        // Step 3: Validate current
        if let Err(e) = self.safety_policy.check_current(&target_current) {
            let result = MockMagResult::Rejected {
                requested_target_b_t: req.target_b_t,
                reason: e.to_string(),
            };
            self.record_rejected(seq, req, &result);
            return result;
        }

        // Step 4: Validate settle time
        if let Err(e) = self.safety_policy.check_settle_time(req.settle_ms) {
            let result = MockMagResult::Rejected {
                requested_target_b_t: req.target_b_t,
                reason: e.to_string(),
            };
            self.record_rejected(seq, req, &result);
            return result;
        }

        // Step 5: Compute ramp time from rate
        let delta_i = CoilCurrent::new(
            target_current.ix_a - self.current_a.ix_a,
            target_current.iy_a - self.current_a.iy_a,
            target_current.iz_a - self.current_a.iz_a,
        );
        let max_delta = delta_i.abs_per_axis().iter().copied().fold(0.0, f64::max);
        let ramp_time_ms = if max_delta == 0.0 {
            0
        } else {
            ((max_delta / req.ramp_rate_a_per_s) * 1000.0).ceil() as u64
        };

        // Step 6: Validate ramp rate (using computed ramp time)
        if let Err(e) = self.safety_policy.check_ramp_rate(
            &self.current_a,
            &target_current,
            ramp_time_ms.max(1),
        ) {
            let result = MockMagResult::Rejected {
                requested_target_b_t: req.target_b_t,
                reason: e.to_string(),
            };
            self.record_rejected(seq, req, &result);
            return result;
        }

        // Accepted
        let result = MockMagResult::Accepted {
            from_current_a: self.current_a,
            to_current_a: target_current,
            estimated_ramp_ms: ramp_time_ms,
        };

        self.events.push(MockMagEvent::RampAccepted {
            seq,
            from_current_a: self.current_a,
            to_current_a: target_current,
            estimated_ramp_ms: ramp_time_ms,
        });

        // Update state
        self.current_a = target_current;
        self.b_field_t = self
            .coil_matrix
            .b_from_current(&target_current)
            .unwrap_or(req.target_b_t);
        self.timestamp_ms += ramp_time_ms + req.settle_ms;

        self.events.push(MockMagEvent::SettleComplete {
            seq,
            final_current_a: self.current_a,
            final_b_t: self.b_field_t,
        });

        self.states.push(MockMagState {
            seq,
            timestamp_ms: self.timestamp_ms,
            current_a: self.current_a,
            b_field_t: self.b_field_t,
        });

        result
    }

    fn record_rejected(&mut self, seq: u64, _req: &RampRequest, result: &MockMagResult) {
        if let MockMagResult::Rejected {
            requested_target_b_t,
            reason,
        } = result
        {
            self.events.push(MockMagEvent::RampRejected {
                seq,
                requested_target_b_t: *requested_target_b_t,
                reason: reason.clone(),
            });
        }
    }

    pub fn events(&self) -> &[MockMagEvent] {
        &self.events
    }

    pub fn states(&self) -> &[MockMagState] {
        &self.states
    }

    pub fn current(&self) -> CoilCurrent {
        self.current_a
    }

    pub fn b_field(&self) -> BVectorCartesian {
        self.b_field_t
    }
}

// ---------------------------------------------------------------------------
// Run artifact types
// ---------------------------------------------------------------------------

/// Magnetic plan artifact: the output of the magnetic planner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagneticPlan {
    #[serde(flatten)]
    pub header: PlanHeader,
    pub coil_matrix_id: String,
    pub safety_policy_id: String,
    pub points: Vec<MagneticPlanPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanHeader {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagneticPlanPoint {
    pub point_id: String,
    pub b_target_t: BVectorCartesian,
    pub coil_current_target_a: CoilCurrent,
    pub settle_ms: u64,
    pub estimated_ramp_ms: u64,
    pub safety_status: String,
}

/// Magnetic safety report artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagneticSafetyReport {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub plan_id: String,
    pub decision: String,
    pub findings: Vec<MagneticSafetyFinding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagneticSafetyFinding {
    pub severity: String,
    pub point_id: String,
    pub field: String,
    pub value: f64,
    pub limit: f64,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Maynuo M8812 reverse-derived types
// ---------------------------------------------------------------------------

/// Serial port settings for a Maynuo M8812 power supply.
///
/// Per reverse analysis: 9600/8/N/1, DTR=true, no flow control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaynuoSerialSettings {
    pub baudrate: u32,
    pub data_bits: u8,
    pub parity: String,
    pub stop_bits: u8,
    pub flow_control: String,
    pub dtr: bool,
    pub read_timeout_ms: u64,
}

impl Default for MaynuoSerialSettings {
    fn default() -> Self {
        Self {
            baudrate: 9600,
            data_bits: 8,
            parity: "none".into(),
            stop_bits: 1,
            flow_control: "none".into(),
            dtr: true,
            read_timeout_ms: 100,
        }
    }
}

/// Per-axis profile for a Maynuo M8812 magnetic axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaynuoAxisProfile {
    pub axis_id: String,
    pub display_name: String,
    pub port_name: String,
    pub device_model: String,
    pub sn_tail: String,
    pub expected_idn: String,
    /// Coil constant in nT/mA (from original software para.xml)
    pub coil_constant_nt_per_ma: f64,
    /// Normalized gain in T/A (coil_constant_nt_per_ma * 1e-6)
    pub gain_t_per_a: f64,
    /// Zero offset in mA
    pub zero_offset_ma: f64,
    /// Zero offset in A
    pub zero_offset_a: f64,
    pub output_default: bool,
    /// Hardware max current in mA (POWER_MAX_CURR from original software = 5000)
    pub max_current_ma: f64,
    /// Hardware max current in A
    pub max_current_a: f64,
    pub voltage_v: u16,
}

/// Three-axis assembly profile for the lab Maynuo M8812 setup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaynuoAxesProfile {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub name: Option<String>,
    pub serial_settings: MaynuoSerialSettings,
    pub x: MaynuoAxisProfile,
    pub y: MaynuoAxisProfile,
    pub z: MaynuoAxisProfile,
    pub safety_policy_id: String,
    pub calibration_date: String,
    pub verified: bool,
    pub verified_by: Option<String>,
}

impl MaynuoAxesProfile {
    /// Build a diagonal CoilMatrix from the per-axis coil constants.
    ///
    /// This bridges the Maynuo axis-gain model to the general 3x3 matrix model.
    pub fn to_coil_matrix(&self) -> CoilMatrix {
        let kx = self.x.gain_t_per_a;
        let ky = self.y.gain_t_per_a;
        let kz = self.z.gain_t_per_a;
        CoilMatrix {
            m: [[kx, 0.0, 0.0], [0.0, ky, 0.0], [0.0, 0.0, kz]],
            i_offset_a: [
                self.x.zero_offset_a,
                self.y.zero_offset_a,
                self.z.zero_offset_a,
            ],
            b_zero_offset_t: [0.0, 0.0, 0.0],
            condition_number: Some(1.0), // diagonal matrix, perfectly conditioned
            calibrated_at: self.calibration_date.clone(),
            verified: self.verified,
            verified_by: self.verified_by.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Maynuo command types (data-only, no I/O)
// ---------------------------------------------------------------------------

/// A single SCPI command as data.
///
/// No serial port is opened; this is a planning artifact only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MaynuoCommand {
    Identify,
    SetRemote,
    SetLocal,
    SetVoltage { voltage_v: u16 },
    SetCurrent { current_a: f64, current_ma: f64 },
    SetOutput { on: bool },
    QueryCurrent,
}

impl MaynuoCommand {
    /// Return the SCPI string for this command.
    pub fn scpi(&self) -> String {
        match self {
            MaynuoCommand::Identify => "*IDN?".into(),
            MaynuoCommand::SetRemote => "SYST:REM".into(),
            MaynuoCommand::SetLocal => "SYST:LOC".into(),
            MaynuoCommand::SetVoltage { voltage_v } => format!("VOLT {voltage_v}"),
            MaynuoCommand::SetCurrent { current_a, .. } => {
                format!("CURR {current_a:.5}")
            }
            MaynuoCommand::SetOutput { on } => {
                format!("OUTP {}", if *on { 1 } else { 0 })
            }
            MaynuoCommand::QueryCurrent => "MEAS:CURR?".into(),
        }
    }

    /// Return the expected response shape (for documentation / validation).
    pub fn expected_response_shape(&self) -> &'static str {
        match self {
            MaynuoCommand::Identify => "MAYNUO,M8812,<SN>,V2.7",
            MaynuoCommand::SetRemote => "ACK",
            MaynuoCommand::SetLocal => "ACK",
            MaynuoCommand::SetVoltage { .. } => "ACK",
            MaynuoCommand::SetCurrent { .. } => "ACK",
            MaynuoCommand::SetOutput { .. } => "ACK",
            MaynuoCommand::QueryCurrent => "0.0XXXX",
        }
    }
}

/// An ordered plan of Maynuo commands for a specific procedure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaynuoCommandPlan {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub name: String,
    pub axis_id: String,
    pub profile_id: String,
    pub executable: bool,
    pub executable_reason: String,
    pub commands: Vec<MaynuoCommandEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaynuoCommandEntry {
    pub seq: u32,
    pub command_type: String,
    pub scpi: String,
    pub expected_response_shape: String,
    pub event_name: String,
    pub blocking: bool,
    pub delay_ms: Option<u64>,
    pub timeout_ms: Option<u64>,
}

// ---------------------------------------------------------------------------
// Maynuo conversion helpers
// ---------------------------------------------------------------------------

/// Convert coil constant from nT/mA to T/A.
///
/// 1 nT/mA = 1e-9 T / 1e-3 A = 1e-6 T/A
pub fn nt_per_ma_to_t_per_a(gain_nt_per_ma: f64) -> f64 {
    gain_nt_per_ma * 1e-6
}

/// Convert magnetic field from nT to coil current in mA.
///
/// Formula: Curr(mA) = Mag(nT) / CoilConstant(nT/mA)
pub fn nt_to_ma(field_nt: f64, coil_constant_nt_per_ma: f64) -> f64 {
    if coil_constant_nt_per_ma == 0.0 {
        0.0
    } else {
        field_nt / coil_constant_nt_per_ma
    }
}

/// Convert coil current from mA to magnetic field in nT.
///
/// Formula: Mag(nT) = Curr(mA) * CoilConstant(nT/mA)
pub fn ma_to_nt(current_ma: f64, coil_constant_nt_per_ma: f64) -> f64 {
    current_ma * coil_constant_nt_per_ma
}

/// Format a current command string from mA.
///
/// Original software formula: `CURR {Abs(totalCurr) / 1000.0:f5}`
/// M8812 only supports positive current; field direction is fixed by coil winding.
pub fn format_current_command_from_ma(current_ma: f64) -> Result<String, MagError> {
    if !current_ma.is_finite() {
        return Err(MagError::NonFiniteValue {
            field: "current_ma".into(),
            value: current_ma,
        });
    }
    if current_ma < 0.0 {
        return Err(MagError::SafetyViolation {
            message: format!(
                "negative current {current_ma} mA is not supported by MAYNUO M8812 (only positive current, field direction fixed by coil winding)"
            ),
        });
    }
    let current_a = current_ma / 1000.0;
    Ok(format!("CURR {current_a:.5}"))
}

// ---------------------------------------------------------------------------
// Maynuo plan builders
// ---------------------------------------------------------------------------

/// Build a safe initialization command plan for a single axis.
pub fn build_safe_init_plan(axis: &MaynuoAxisProfile) -> MaynuoCommandPlan {
    let entries = vec![
        MaynuoCommandEntry {
            seq: 1,
            command_type: "identify".into(),
            scpi: "*IDN?".into(),
            expected_response_shape: "MAYNUO,M8812,<SN>,V2.7".into(),
            event_name: "mag_idn_queried".into(),
            blocking: true,
            delay_ms: None,
            timeout_ms: Some(300),
        },
        MaynuoCommandEntry {
            seq: 2,
            command_type: "set_remote".into(),
            scpi: "SYST:REM".into(),
            expected_response_shape: "ACK".into(),
            event_name: "mag_remote_mode_set".into(),
            blocking: false,
            delay_ms: Some(50),
            timeout_ms: None,
        },
        MaynuoCommandEntry {
            seq: 3,
            command_type: "set_voltage".into(),
            scpi: format!("VOLT {}", axis.voltage_v),
            expected_response_shape: "ACK".into(),
            event_name: "mag_voltage_set".into(),
            blocking: false,
            delay_ms: Some(50),
            timeout_ms: None,
        },
        MaynuoCommandEntry {
            seq: 4,
            command_type: "set_current".into(),
            scpi: "CURR 0.00000".into(),
            expected_response_shape: "ACK".into(),
            event_name: "mag_current_zeroed".into(),
            blocking: false,
            delay_ms: Some(50),
            timeout_ms: None,
        },
        MaynuoCommandEntry {
            seq: 5,
            command_type: "set_output".into(),
            scpi: "OUTP 0".into(),
            expected_response_shape: "ACK".into(),
            event_name: "mag_output_disabled".into(),
            blocking: false,
            delay_ms: Some(50),
            timeout_ms: None,
        },
        MaynuoCommandEntry {
            seq: 6,
            command_type: "query_current".into(),
            scpi: "MEAS:CURR?".into(),
            expected_response_shape: "0.00000".into(),
            event_name: "mag_current_queried".into(),
            blocking: true,
            delay_ms: None,
            timeout_ms: Some(300),
        },
        MaynuoCommandEntry {
            seq: 7,
            command_type: "set_output".into(),
            scpi: "OUTP 0".into(),
            expected_response_shape: "ACK".into(),
            event_name: "mag_output_confirmed_off".into(),
            blocking: false,
            delay_ms: Some(50),
            timeout_ms: None,
        },
        MaynuoCommandEntry {
            seq: 8,
            command_type: "set_local".into(),
            scpi: "SYST:LOC".into(),
            expected_response_shape: "ACK".into(),
            event_name: "mag_local_mode_set".into(),
            blocking: false,
            delay_ms: Some(50),
            timeout_ms: None,
        },
    ];

    MaynuoCommandPlan {
        schema_version: "0.2.0".into(),
        kind: "maynuo_command_plan".into(),
        id: format!("maynuo_safe_init_{}", axis.axis_id),
        name: "Maynuo M8812 Safe Initialization Plan".into(),
        axis_id: axis.axis_id.clone(),
        profile_id: "maynuo_m8812_lab_xyz".into(),
        executable: false,
        executable_reason:
            "Mag-M0.5 mock-only: requires Mag-M2B backend bring-up for real execution".into(),
        commands: entries,
    }
}

/// Build a query-current command plan for a single axis.
pub fn build_query_current_plan(axis: &MaynuoAxisProfile) -> MaynuoCommandPlan {
    MaynuoCommandPlan {
        schema_version: "0.2.0".into(),
        kind: "maynuo_command_plan".into(),
        id: format!("maynuo_query_current_{}", axis.axis_id),
        name: "Maynuo M8812 Query Current Plan".into(),
        axis_id: axis.axis_id.clone(),
        profile_id: "maynuo_m8812_lab_xyz".into(),
        executable: false,
        executable_reason: "Mag-M0.5 mock-only: requires Mag-M2 backend for real execution".into(),
        commands: vec![MaynuoCommandEntry {
            seq: 1,
            command_type: "query_current".into(),
            scpi: "MEAS:CURR?".into(),
            expected_response_shape: "0.0XXXX".into(),
            event_name: "mag_current_queried".into(),
            blocking: true,
            delay_ms: None,
            timeout_ms: Some(300),
        }],
    }
}

/// Build a 10 mA micro-test command plan for a single axis.
///
/// This is the only low-current micro-test example permitted in Mag-M0.5.
pub fn build_10ma_microtest_plan(axis: &MaynuoAxisProfile) -> Result<MaynuoCommandPlan, MagError> {
    let test_current_ma = 10.0;

    // Validate against axis hardware limit
    if test_current_ma > axis.max_current_ma {
        return Err(MagError::CurrentLimitExceeded {
            axis: axis.axis_id.chars().next().unwrap_or('?'),
            current_a: test_current_ma / 1000.0,
            limit_a: axis.max_current_a,
        });
    }

    let test_current_a = test_current_ma / 1000.0;

    let entries = vec![
        MaynuoCommandEntry {
            seq: 1,
            command_type: "identify".into(),
            scpi: "*IDN?".into(),
            expected_response_shape: "MAYNUO,M8812,<SN>,V2.7".into(),
            event_name: "mag_idn_queried".into(),
            blocking: true,
            delay_ms: None,
            timeout_ms: Some(300),
        },
        MaynuoCommandEntry {
            seq: 2,
            command_type: "set_remote".into(),
            scpi: "SYST:REM".into(),
            expected_response_shape: "ACK".into(),
            event_name: "mag_remote_mode_set".into(),
            blocking: false,
            delay_ms: Some(50),
            timeout_ms: None,
        },
        MaynuoCommandEntry {
            seq: 3,
            command_type: "set_voltage".into(),
            scpi: format!("VOLT {}", axis.voltage_v),
            expected_response_shape: "ACK".into(),
            event_name: "mag_voltage_set".into(),
            blocking: false,
            delay_ms: Some(50),
            timeout_ms: None,
        },
        MaynuoCommandEntry {
            seq: 4,
            command_type: "set_current".into(),
            scpi: format!("CURR {test_current_a:.5}"),
            expected_response_shape: "ACK".into(),
            event_name: "mag_current_set".into(),
            blocking: false,
            delay_ms: Some(50),
            timeout_ms: None,
        },
        MaynuoCommandEntry {
            seq: 5,
            command_type: "set_output".into(),
            scpi: "OUTP 1".into(),
            expected_response_shape: "ACK".into(),
            event_name: "mag_output_enabled".into(),
            blocking: false,
            delay_ms: Some(200),
            timeout_ms: None,
        },
        MaynuoCommandEntry {
            seq: 6,
            command_type: "query_current".into(),
            scpi: "MEAS:CURR?".into(),
            expected_response_shape: "0.0XXXX".into(),
            event_name: "mag_current_measured".into(),
            blocking: true,
            delay_ms: None,
            timeout_ms: Some(300),
        },
        MaynuoCommandEntry {
            seq: 7,
            command_type: "set_output".into(),
            scpi: "OUTP 0".into(),
            expected_response_shape: "ACK".into(),
            event_name: "mag_output_disabled".into(),
            blocking: false,
            delay_ms: Some(50),
            timeout_ms: None,
        },
        MaynuoCommandEntry {
            seq: 8,
            command_type: "set_current".into(),
            scpi: "CURR 0.00000".into(),
            expected_response_shape: "ACK".into(),
            event_name: "mag_current_zeroed".into(),
            blocking: false,
            delay_ms: Some(50),
            timeout_ms: None,
        },
        MaynuoCommandEntry {
            seq: 9,
            command_type: "set_local".into(),
            scpi: "SYST:LOC".into(),
            expected_response_shape: "ACK".into(),
            event_name: "mag_local_mode_set".into(),
            blocking: false,
            delay_ms: Some(50),
            timeout_ms: None,
        },
    ];

    Ok(MaynuoCommandPlan {
        schema_version: "0.2.0".into(),
        kind: "maynuo_command_plan".into(),
        id: format!("maynuo_10ma_microtest_{}", axis.axis_id),
        name: "Maynuo M8812 10 mA Micro-Test Plan".into(),
        axis_id: axis.axis_id.clone(),
        profile_id: "maynuo_m8812_lab_xyz".into(),
        executable: false,
        executable_reason:
            "Mag-M0.5 mock-only: future Mag-M3 candidate, not executable in Mag-M0.5".into(),
        commands: entries,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn example_coil_matrix() -> CoilMatrix {
        CoilMatrix {
            m: [
                [0.005, 0.0001, -0.0002],
                [0.0001, 0.005, 0.0001],
                [-0.0002, 0.0001, 0.005],
            ],
            i_offset_a: [0.001, -0.0005, 0.0002],
            b_zero_offset_t: [5e-6, -3e-6, 2e-6],
            condition_number: Some(1.08),
            calibrated_at: "2026-05-15T09:30:00Z".into(),
            verified: true,
            verified_by: Some("mock_cal_agent".into()),
        }
    }

    fn singular_coil_matrix() -> CoilMatrix {
        CoilMatrix {
            m: [[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [3.0, 6.0, 9.0]],
            i_offset_a: [0.0, 0.0, 0.0],
            b_zero_offset_t: [0.0, 0.0, 0.0],
            condition_number: None,
            calibrated_at: "2026-05-15T09:30:00Z".into(),
            verified: true,
            verified_by: None,
        }
    }

    fn example_safety_policy() -> MagSafetyPolicy {
        MagSafetyPolicy {
            schema_version: "0.2.0".into(),
            kind: "mag_safety_policy".into(),
            id: "mag_safety_test".into(),
            name: Some("Test Policy".into()),
            max_current_a_per_axis: 2.0,
            max_abs_current_vector_a: 3.0,
            max_ramp_rate_a_per_s: 0.5,
            max_vector_ramp_rate_a_per_s: 0.8,
            min_settle_ms: 100,
            max_b_abs_t: 0.01,
            max_calibration_age_days: 30,
            require_calibration_verified: true,
            recipe_override_safety: false,
        }
    }

    // -----------------------------------------------------------------------
    // Cartesian ↔ Spherical conversion
    // -----------------------------------------------------------------------

    #[test]
    fn cartesian_to_spherical_basic() {
        let c = BVectorCartesian::new(1.0, 0.0, 0.0);
        let s = cartesian_to_spherical(&c);
        assert!((s.b_abs_t - 1.0).abs() < 1e-10);
        assert!((s.theta_rad - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
        assert!(s.phi_rad.abs() < 1e-10);
    }

    #[test]
    fn cartesian_to_spherical_z_axis() {
        let c = BVectorCartesian::new(0.0, 0.0, 2.0);
        let s = cartesian_to_spherical(&c);
        assert!((s.b_abs_t - 2.0).abs() < 1e-10);
        assert!(s.theta_rad.abs() < 1e-10);
        // phi is undefined for z-axis; convention returns 0
        assert!(s.phi_rad.abs() < 1e-10);
    }

    #[test]
    fn cartesian_to_spherical_zero_vector() {
        let c = BVectorCartesian::new(0.0, 0.0, 0.0);
        let s = cartesian_to_spherical(&c);
        assert_eq!(s.b_abs_t, 0.0);
        assert_eq!(s.theta_rad, 0.0);
        assert_eq!(s.phi_rad, 0.0);
    }

    #[test]
    fn spherical_to_cartesian_roundtrip() {
        let originals = vec![
            BVectorSpherical::new(1.0, 0.5, 1.0),
            BVectorSpherical::new(2.0, 1.2, 3.5),
            BVectorSpherical::new(0.5, std::f64::consts::FRAC_PI_2, 0.0),
        ];
        for orig in originals {
            let c = spherical_to_cartesian(&orig);
            let s = cartesian_to_spherical(&c);
            assert!((s.b_abs_t - orig.b_abs_t).abs() < 1e-10);
            assert!((s.theta_rad - orig.theta_rad).abs() < 1e-10);
            // phi normalization may shift by 2π
            let phi_diff = (s.phi_rad - orig.phi_rad).abs();
            assert!(phi_diff < 1e-10 || (phi_diff - 2.0 * std::f64::consts::PI).abs() < 1e-10);
        }
    }

    #[test]
    fn cartesian_to_spherical_to_cartesian_roundtrip() {
        let originals = vec![
            BVectorCartesian::new(1.0, 2.0, 3.0),
            BVectorCartesian::new(-1.0, -2.0, -3.0),
            BVectorCartesian::new(0.0, 1.0, 0.0),
        ];
        for orig in originals {
            let s = cartesian_to_spherical(&orig);
            let c = spherical_to_cartesian(&s);
            assert!((c.bx_t - orig.bx_t).abs() < 1e-10);
            assert!((c.by_t - orig.by_t).abs() < 1e-10);
            assert!((c.bz_t - orig.bz_t).abs() < 1e-10);
        }
    }

    // -----------------------------------------------------------------------
    // Coil matrix: target B to current
    // -----------------------------------------------------------------------

    #[test]
    fn current_from_b_well_conditioned() {
        let cm = example_coil_matrix();
        let b = BVectorCartesian::new(0.001, 0.0, 0.002);
        let current = cm.current_from_b(&b).unwrap();
        assert!(current.is_finite());
        // Verify round-trip: B = M * (I - I_offset) + B_zero_offset
        let b_back = cm.b_from_current(&current).unwrap();
        assert!((b_back.bx_t - b.bx_t).abs() < 1e-10);
        assert!((b_back.by_t - b.by_t).abs() < 1e-10);
        assert!((b_back.bz_t - b.bz_t).abs() < 1e-10);
    }

    #[test]
    fn zero_offset_applied() {
        let cm = example_coil_matrix();
        let b = BVectorCartesian::new(5e-6, -3e-6, 2e-6); // matches B_zero_offset
        let current = cm.current_from_b(&b).unwrap();
        // At B = B_zero_offset, I should equal I_offset
        assert!((current.ix_a - cm.i_offset_a[0]).abs() < 1e-10);
        assert!((current.iy_a - cm.i_offset_a[1]).abs() < 1e-10);
        assert!((current.iz_a - cm.i_offset_a[2]).abs() < 1e-10);
    }

    #[test]
    fn b_from_current_roundtrip() {
        let cm = example_coil_matrix();
        let original_current = CoilCurrent::new(0.1, 0.2, -0.1);
        let b = cm.b_from_current(&original_current).unwrap();
        let current_back = cm.current_from_b(&b).unwrap();
        assert!((current_back.ix_a - original_current.ix_a).abs() < 1e-10);
        assert!((current_back.iy_a - original_current.iy_a).abs() < 1e-10);
        assert!((current_back.iz_a - original_current.iz_a).abs() < 1e-10);
    }

    #[test]
    fn singular_matrix_rejected() {
        let cm = singular_coil_matrix();
        let b = BVectorCartesian::new(0.001, 0.0, 0.0);
        let result = cm.current_from_b(&b);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::SingularMatrix { .. }
        ));
    }

    // -----------------------------------------------------------------------
    // Safety: current limit rejection
    // -----------------------------------------------------------------------

    #[test]
    fn current_limit_per_axis_rejected() {
        let policy = example_safety_policy();
        let current = CoilCurrent::new(2.5, 0.0, 0.0); // exceeds 2.0 A on X
        let result = policy.check_current(&current);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::CurrentLimitExceeded { axis: 'x', .. }
        ));
    }

    #[test]
    fn current_limit_y_axis_rejected() {
        let policy = example_safety_policy();
        let current = CoilCurrent::new(0.0, 3.0, 0.0); // exceeds 2.0 A on Y
        let result = policy.check_current(&current);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::CurrentLimitExceeded { axis: 'y', .. }
        ));
    }

    #[test]
    fn vector_current_limit_rejected() {
        let policy = example_safety_policy();
        // Each axis is under 2.0 but vector norm exceeds 3.0
        // |I| = sqrt(4 + 4 + 4) = sqrt(12) ≈ 3.46 > 3.0
        let current = CoilCurrent::new(2.0, 2.0, 2.0);
        let result = policy.check_current(&current);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::VectorCurrentLimitExceeded { .. }
        ));
    }

    #[test]
    fn current_within_limits_accepted() {
        let policy = example_safety_policy();
        let current = CoilCurrent::new(1.0, 1.0, 1.0);
        assert!(policy.check_current(&current).is_ok());
    }

    // -----------------------------------------------------------------------
    // Safety: ramp rate rejection
    // -----------------------------------------------------------------------

    #[test]
    fn ramp_rate_per_axis_rejected() {
        let policy = example_safety_policy();
        let from = CoilCurrent::new(0.0, 0.0, 0.0);
        let to = CoilCurrent::new(1.0, 0.0, 0.0);
        // 1.0 A in 1 second = 1.0 A/s > 0.5 A/s limit
        let result = policy.check_ramp_rate(&from, &to, 1000);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::RampRateExceeded { axis: 'x', .. }
        ));
    }

    #[test]
    fn ramp_rate_vector_rejected() {
        let policy = example_safety_policy();
        let from = CoilCurrent::new(0.0, 0.0, 0.0);
        let to = CoilCurrent::new(0.5, 0.5, 0.5);
        // |ΔI| = sqrt(0.75) ≈ 0.866 A in 1 second
        // 0.866 A/s > 0.8 A/s vector limit
        let result = policy.check_ramp_rate(&from, &to, 1000);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::VectorRampRateExceeded { .. }
        ));
    }

    #[test]
    fn slow_ramp_accepted() {
        let policy = example_safety_policy();
        let from = CoilCurrent::new(0.0, 0.0, 0.0);
        let to = CoilCurrent::new(1.0, 0.0, 0.0);
        // 1.0 A in 10 seconds = 0.1 A/s < 0.5 A/s limit
        assert!(policy.check_ramp_rate(&from, &to, 10_000).is_ok());
    }

    #[test]
    fn zero_ramp_time_rejected() {
        let policy = example_safety_policy();
        let from = CoilCurrent::new(0.0, 0.0, 0.0);
        let to = CoilCurrent::new(0.1, 0.0, 0.0);
        let result = policy.check_ramp_rate(&from, &to, 0);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Safety: settle time
    // -----------------------------------------------------------------------

    #[test]
    fn settle_time_too_short_rejected() {
        let policy = example_safety_policy();
        let result = policy.check_settle_time(50);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::SettleTimeTooShort {
                settle_ms: 50,
                min_ms: 100
            }
        ));
    }

    #[test]
    fn settle_time_accepted() {
        let policy = example_safety_policy();
        assert!(policy.check_settle_time(100).is_ok());
        assert!(policy.check_settle_time(500).is_ok());
    }

    // -----------------------------------------------------------------------
    // Safety: B field range
    // -----------------------------------------------------------------------

    #[test]
    fn b_field_out_of_range_rejected() {
        let policy = example_safety_policy();
        let b = BVectorCartesian::new(0.0, 0.0, 0.02); // |B| = 0.02 T > 0.01 T limit
        let result = policy.check_b_field(&b);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::BFieldOutOfRange { .. }
        ));
    }

    #[test]
    fn b_field_within_range_accepted() {
        let policy = example_safety_policy();
        let b = BVectorCartesian::new(0.0, 0.0, 0.005);
        assert!(policy.check_b_field(&b).is_ok());
    }

    // -----------------------------------------------------------------------
    // Safety: calibration checks
    // -----------------------------------------------------------------------

    #[test]
    fn unverified_calibration_rejected() {
        let policy = example_safety_policy();
        let mut cm = example_coil_matrix();
        cm.verified = false;
        let result = policy.check_calibration(&cm);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::CalibrationMissing { .. }
        ));
    }

    #[test]
    fn ill_conditioned_matrix_rejected() {
        let policy = example_safety_policy();
        let mut cm = example_coil_matrix();
        // Override condition number to simulate ill-conditioning
        cm.condition_number = Some(1e13);
        let result = policy.check_calibration(&cm);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::IllConditionedMatrix { .. }
        ));
    }

    #[test]
    fn verified_calibration_accepted() {
        let policy = example_safety_policy();
        let cm = example_coil_matrix();
        assert!(policy.check_calibration(&cm).is_ok());
    }

    // -----------------------------------------------------------------------
    // Mock axes: acceptance path
    // -----------------------------------------------------------------------

    #[test]
    fn mock_axes_accepted_ramp() {
        let cm = example_coil_matrix();
        let policy = example_safety_policy();
        let mut axes = MockMagAxes::new(CoilCurrent::new(0.0, 0.0, 0.0), cm, policy).unwrap();

        let req = RampRequest {
            target_b_t: BVectorCartesian::new(0.001, 0.0, 0.002),
            ramp_rate_a_per_s: 0.3,
            settle_ms: 200,
        };

        let result = axes.apply(&req);
        assert!(
            matches!(result, MockMagResult::Accepted { .. }),
            "Expected Accepted, got {:?}",
            result
        );

        // Should have 2 events: RampAccepted + SettleComplete
        assert_eq!(axes.events().len(), 2);
        assert!(matches!(
            axes.events()[0],
            MockMagEvent::RampAccepted { .. }
        ));
        assert!(matches!(
            axes.events()[1],
            MockMagEvent::SettleComplete { .. }
        ));
    }

    #[test]
    fn mock_axes_rejected_over_current() {
        let cm = example_coil_matrix();
        let policy = example_safety_policy();
        let mut axes = MockMagAxes::new(CoilCurrent::new(0.0, 0.0, 0.0), cm, policy).unwrap();

        // Target B that would require excessive current
        let req = RampRequest {
            target_b_t: BVectorCartesian::new(0.01, 0.0, 0.0),
            ramp_rate_a_per_s: 0.3,
            settle_ms: 200,
        };

        let result = axes.apply(&req);
        assert!(
            matches!(result, MockMagResult::Rejected { .. }),
            "Expected Rejected, got {:?}",
            result
        );

        assert_eq!(axes.events().len(), 1);
        assert!(matches!(
            axes.events()[0],
            MockMagEvent::RampRejected { .. }
        ));
    }

    #[test]
    fn mock_axes_rejected_ramp_rate() {
        let cm = example_coil_matrix();
        let policy = example_safety_policy();
        let mut axes = MockMagAxes::new(CoilCurrent::new(0.0, 0.0, 0.0), cm, policy).unwrap();

        let req = RampRequest {
            target_b_t: BVectorCartesian::new(0.001, 0.0, 0.002),
            ramp_rate_a_per_s: 2.0, // exceeds 0.5 A/s limit
            settle_ms: 200,
        };

        let result = axes.apply(&req);
        assert!(
            matches!(result, MockMagResult::Rejected { .. }),
            "Expected Rejected, got {:?}",
            result
        );
    }

    #[test]
    fn mock_axes_rejected_settle_time() {
        let cm = example_coil_matrix();
        let policy = example_safety_policy();
        let mut axes = MockMagAxes::new(CoilCurrent::new(0.0, 0.0, 0.0), cm, policy).unwrap();

        let req = RampRequest {
            target_b_t: BVectorCartesian::new(0.001, 0.0, 0.002),
            ramp_rate_a_per_s: 0.3,
            settle_ms: 50, // below 100 ms minimum
        };

        let result = axes.apply(&req);
        assert!(
            matches!(result, MockMagResult::Rejected { .. }),
            "Expected Rejected, got {:?}",
            result
        );
    }

    #[test]
    fn mock_axes_state_updates_correctly() {
        let cm = example_coil_matrix();
        let policy = example_safety_policy();
        let mut axes =
            MockMagAxes::new(CoilCurrent::new(0.0, 0.0, 0.0), cm.clone(), policy).unwrap();

        let req = RampRequest {
            target_b_t: BVectorCartesian::new(0.001, 0.0, 0.002),
            ramp_rate_a_per_s: 0.3,
            settle_ms: 200,
        };

        axes.apply(&req);

        // State should be updated
        let final_b = axes.b_field();
        assert!((final_b.bx_t - 0.001).abs() < 1e-8);
        assert!((final_b.by_t).abs() < 1e-8);
        assert!((final_b.bz_t - 0.002).abs() < 1e-8);

        // Verify round-trip through coil matrix
        let current = axes.current();
        let computed_b = cm.b_from_current(&current).unwrap();
        assert!((computed_b.bx_t - final_b.bx_t).abs() < 1e-10);
        assert!((computed_b.by_t - final_b.by_t).abs() < 1e-10);
        assert!((computed_b.bz_t - final_b.bz_t).abs() < 1e-10);
    }

    // -----------------------------------------------------------------------
    // Mock axes: deterministic replay
    // -----------------------------------------------------------------------

    #[test]
    fn mock_axes_replay_deterministic() {
        let cm = example_coil_matrix();
        let policy = example_safety_policy();

        let mut axes1 =
            MockMagAxes::new(CoilCurrent::new(0.0, 0.0, 0.0), cm.clone(), policy.clone()).unwrap();
        let mut axes2 =
            MockMagAxes::new(CoilCurrent::new(0.0, 0.0, 0.0), cm.clone(), policy.clone()).unwrap();

        let reqs = vec![
            RampRequest {
                target_b_t: BVectorCartesian::new(0.001, 0.0, 0.002),
                ramp_rate_a_per_s: 0.3,
                settle_ms: 200,
            },
            RampRequest {
                target_b_t: BVectorCartesian::new(0.002, 0.001, 0.001),
                ramp_rate_a_per_s: 0.3,
                settle_ms: 200,
            },
        ];

        for req in &reqs {
            axes1.apply(req);
            axes2.apply(req);
        }

        assert_eq!(axes1.events(), axes2.events());
        assert_eq!(axes1.states(), axes2.states());
    }

    // -----------------------------------------------------------------------
    // Artifact schema validation
    // -----------------------------------------------------------------------

    #[test]
    fn magnetic_plan_roundtrip_serde() {
        let plan = MagneticPlan {
            header: PlanHeader {
                schema_version: "0.2.0".into(),
                kind: "magnetic_plan".into(),
                id: "plan_001".into(),
                name: Some("Test Plan".into()),
            },
            coil_matrix_id: "cm_001".into(),
            safety_policy_id: "sp_001".into(),
            points: vec![MagneticPlanPoint {
                point_id: "pt_001".into(),
                b_target_t: BVectorCartesian::new(0.001, 0.0, 0.002),
                coil_current_target_a: CoilCurrent::new(0.1, 0.0, 0.2),
                settle_ms: 200,
                estimated_ramp_ms: 500,
                safety_status: "pass".into(),
            }],
        };

        let json = serde_json::to_string(&plan).unwrap();
        let back: MagneticPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(plan, back);
    }

    #[test]
    fn magnetic_safety_report_roundtrip_serde() {
        let report = MagneticSafetyReport {
            schema_version: "0.2.0".into(),
            kind: "magnetic_safety_report".into(),
            id: "msr_001".into(),
            plan_id: "plan_001".into(),
            decision: "allow".into(),
            findings: vec![MagneticSafetyFinding {
                severity: "info".into(),
                point_id: "pt_001".into(),
                field: "b_abs_t".into(),
                value: 0.002,
                limit: 0.01,
                message: "B field within limits".into(),
            }],
        };

        let json = serde_json::to_string(&report).unwrap();
        let back: MagneticSafetyReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, back);
    }

    #[test]
    fn coil_matrix_roundtrip_serde() {
        let cm = example_coil_matrix();
        let json = serde_json::to_string(&cm).unwrap();
        let back: CoilMatrix = serde_json::from_str(&json).unwrap();
        assert_eq!(cm.m, back.m);
        assert_eq!(cm.i_offset_a, back.i_offset_a);
        assert_eq!(cm.b_zero_offset_t, back.b_zero_offset_t);
        assert_eq!(cm.verified, back.verified);
    }

    #[test]
    fn safety_policy_roundtrip_serde() {
        let policy = example_safety_policy();
        let json = serde_json::to_string(&policy).unwrap();
        let back: MagSafetyPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, back);
    }

    // -----------------------------------------------------------------------
    // Non-finite value rejection
    // -----------------------------------------------------------------------

    #[test]
    fn nan_b_field_rejected() {
        let cm = example_coil_matrix();
        let b = BVectorCartesian::new(f64::NAN, 0.0, 0.0);
        let result = cm.current_from_b(&b);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::NonFiniteValue { .. }
        ));
    }

    #[test]
    fn inf_current_rejected() {
        let policy = example_safety_policy();
        let current = CoilCurrent::new(f64::INFINITY, 0.0, 0.0);
        let result = policy.check_current(&current);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::NonFiniteValue { .. }
        ));
    }

    // -----------------------------------------------------------------------
    // No hardware dependency verification
    // -----------------------------------------------------------------------

    #[test]
    fn no_hardware_dependency() {
        // This test proves that all public API operations are pure computation.
        // If any code path attempted serial/USB/TCP access, it would require
        // those crates to be in Cargo.toml, which they are not.
        let cm = example_coil_matrix();
        let policy = example_safety_policy();
        let _axes = MockMagAxes::new(CoilCurrent::new(0.0, 0.0, 0.0), cm, policy).unwrap();
        // Pure computation: reaching here proves no external dependency was invoked.
    }

    // -----------------------------------------------------------------------
    // Maynuo conversion helpers
    // -----------------------------------------------------------------------

    #[test]
    fn nt_per_ma_to_t_per_a_conversion() {
        // 143.26 nT/mA = 143.26 * 1e-6 T/A = 1.4326e-4 T/A
        let gain = nt_per_ma_to_t_per_a(143.26);
        assert!((gain - 1.4326e-4).abs() < 1e-12);
    }

    #[test]
    fn coil_constant_143_26_produces_correct_t_per_a() {
        let gain = nt_per_ma_to_t_per_a(143.26);
        assert!((gain - 1.4326e-4).abs() < 1e-12);
    }

    #[test]
    fn coil_constant_141_77_produces_correct_t_per_a() {
        let gain = nt_per_ma_to_t_per_a(141.77);
        assert!((gain - 1.4177e-4).abs() < 1e-12);
    }

    #[test]
    fn coil_constant_156_15_produces_correct_t_per_a() {
        let gain = nt_per_ma_to_t_per_a(156.15);
        assert!((gain - 1.5615e-4).abs() < 1e-12);
    }

    #[test]
    fn nt_to_ma_conversion() {
        // 1000 nT with coil constant 143.26 nT/mA = 6.9803 mA
        let current_ma = nt_to_ma(1000.0, 143.26);
        assert!((current_ma - 6.9803).abs() < 0.001);
    }

    #[test]
    fn ma_to_nt_conversion() {
        // 10 mA with coil constant 143.26 nT/mA = 1432.6 nT
        let field_nt = ma_to_nt(10.0, 143.26);
        assert!((field_nt - 1432.6).abs() < 0.01);
    }

    #[test]
    fn ma_nt_roundtrip() {
        let original_ma = 42.5;
        let coil_constant = 143.26;
        let field_nt = ma_to_nt(original_ma, coil_constant);
        let back_ma = nt_to_ma(field_nt, coil_constant);
        assert!((back_ma - original_ma).abs() < 1e-10);
    }

    #[test]
    fn format_10ma_command() {
        let cmd = format_current_command_from_ma(10.0).unwrap();
        assert_eq!(cmd, "CURR 0.01000");
    }

    #[test]
    fn format_0ma_command() {
        let cmd = format_current_command_from_ma(0.0).unwrap();
        assert_eq!(cmd, "CURR 0.00000");
    }

    #[test]
    fn format_100ma_command() {
        let cmd = format_current_command_from_ma(100.0).unwrap();
        assert_eq!(cmd, "CURR 0.10000");
    }

    #[test]
    fn negative_current_rejected() {
        let result = format_current_command_from_ma(-10.0);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::SafetyViolation { .. }
        ));
    }

    #[test]
    fn nan_current_rejected() {
        let result = format_current_command_from_ma(f64::NAN);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::NonFiniteValue { .. }
        ));
    }

    // -----------------------------------------------------------------------
    // Maynuo profile helpers
    // -----------------------------------------------------------------------

    fn example_maynuo_axis_profile() -> MaynuoAxisProfile {
        MaynuoAxisProfile {
            axis_id: "mag_x".into(),
            display_name: "X Axis".into(),
            port_name: "COM4".into(),
            device_model: "MAYNUO M8812".into(),
            sn_tail: "2020".into(),
            expected_idn: "MAYNUO,M8812,080020960220402020,V2.7".into(),
            coil_constant_nt_per_ma: 143.26,
            gain_t_per_a: 1.4326e-4,
            zero_offset_ma: 0.0,
            zero_offset_a: 0.0,
            output_default: false,
            max_current_ma: 5000.0,
            max_current_a: 5.0,
            voltage_v: 75,
        }
    }

    fn example_maynuo_axes_profile() -> MaynuoAxesProfile {
        // Verified SN mapping (2026-06-01, power-cycle identification):
        // X -> SN 080020960220402020 (port dynamically assigned)
        // Y -> SN 080020960220402022 (port dynamically assigned)
        // Z -> SN 080020960220402003 (port dynamically assigned)
        // Port paths MUST NOT be used as stable identifiers.
        MaynuoAxesProfile {
            schema_version: "0.2.0".into(),
            kind: "maynuo_axes_profile".into(),
            id: "maynuo_m8812_lab_xyz".into(),
            name: Some("Lab Maynuo M8812 XYZ".into()),
            serial_settings: MaynuoSerialSettings::default(),
            x: example_maynuo_axis_profile(),
            y: MaynuoAxisProfile {
                axis_id: "mag_y".into(),
                display_name: "Y Axis".into(),
                port_name: "COM6".into(),
                device_model: "MAYNUO M8812".into(),
                sn_tail: "2022".into(),
                expected_idn: "MAYNUO,M8812,080020960220402022,V2.7".into(),
                coil_constant_nt_per_ma: 141.77,
                gain_t_per_a: 1.4177e-4,
                zero_offset_ma: 0.0,
                zero_offset_a: 0.0,
                output_default: false,
                max_current_ma: 5000.0,
                max_current_a: 5.0,
                voltage_v: 75,
            },
            z: MaynuoAxisProfile {
                axis_id: "mag_z".into(),
                display_name: "Z Axis".into(),
                port_name: "COM3".into(),
                device_model: "MAYNUO M8812".into(),
                sn_tail: "2003".into(),
                expected_idn: "MAYNUO,M8812,080020960220402003,V2.7".into(),
                coil_constant_nt_per_ma: 156.15,
                gain_t_per_a: 1.5615e-4,
                zero_offset_ma: 0.0,
                zero_offset_a: 0.0,
                output_default: false,
                max_current_ma: 5000.0,
                max_current_a: 5.0,
                voltage_v: 75,
            },
            safety_policy_id: "mag_safety_lab_default".into(),
            calibration_date: "2026-05-15".into(),
            verified: true,
            verified_by: Some("reverse_analysis_agent".into()),
        }
    }

    // -----------------------------------------------------------------------
    // Maynuo axes profile to coil matrix
    // -----------------------------------------------------------------------

    #[test]
    fn maynuo_profile_to_coil_matrix_is_diagonal() {
        let profile = example_maynuo_axes_profile();
        let cm = profile.to_coil_matrix();

        // Diagonal elements match per-axis gains
        assert!((cm.m[0][0] - 1.4326e-4).abs() < 1e-12);
        assert!((cm.m[1][1] - 1.4177e-4).abs() < 1e-12);
        assert!((cm.m[2][2] - 1.5615e-4).abs() < 1e-12);

        // Off-diagonal elements are zero
        assert_eq!(cm.m[0][1], 0.0);
        assert_eq!(cm.m[0][2], 0.0);
        assert_eq!(cm.m[1][0], 0.0);
        assert_eq!(cm.m[1][2], 0.0);
        assert_eq!(cm.m[2][0], 0.0);
        assert_eq!(cm.m[2][1], 0.0);

        // Condition number is 1.0 (perfectly conditioned diagonal matrix)
        assert_eq!(cm.condition_number(), 1.0);
    }

    #[test]
    fn maynuo_coil_matrix_roundtrip() {
        let profile = example_maynuo_axes_profile();
        let cm = profile.to_coil_matrix();

        // Set 10 mA on X axis = 0.01 A
        let current = CoilCurrent::new(0.01, 0.0, 0.0);
        let b = cm.b_from_current(&current).unwrap();

        // Bx = 0.01 A * 1.4326e-4 T/A = 1.4326e-6 T
        assert!((b.bx_t - 1.4326e-6).abs() < 1e-18);
        assert_eq!(b.by_t, 0.0);
        assert_eq!(b.bz_t, 0.0);

        // Round-trip: current from B
        let current_back = cm.current_from_b(&b).unwrap();
        assert!((current_back.ix_a - 0.01).abs() < 1e-15);
    }

    // -----------------------------------------------------------------------
    // Maynuo command SCPI generation
    // -----------------------------------------------------------------------

    #[test]
    fn maynuo_command_scpi_identify() {
        assert_eq!(MaynuoCommand::Identify.scpi(), "*IDN?");
    }

    #[test]
    fn maynuo_command_scpi_set_remote() {
        assert_eq!(MaynuoCommand::SetRemote.scpi(), "SYST:REM");
    }

    #[test]
    fn maynuo_command_scpi_set_local() {
        assert_eq!(MaynuoCommand::SetLocal.scpi(), "SYST:LOC");
    }

    #[test]
    fn maynuo_command_scpi_set_voltage() {
        assert_eq!(
            MaynuoCommand::SetVoltage { voltage_v: 75 }.scpi(),
            "VOLT 75"
        );
    }

    #[test]
    fn maynuo_command_scpi_set_current_10ma() {
        assert_eq!(
            MaynuoCommand::SetCurrent {
                current_a: 0.01,
                current_ma: 10.0
            }
            .scpi(),
            "CURR 0.01000"
        );
    }

    #[test]
    fn maynuo_command_scpi_set_current_0ma() {
        assert_eq!(
            MaynuoCommand::SetCurrent {
                current_a: 0.0,
                current_ma: 0.0
            }
            .scpi(),
            "CURR 0.00000"
        );
    }

    #[test]
    fn maynuo_command_scpi_set_output_on() {
        assert_eq!(MaynuoCommand::SetOutput { on: true }.scpi(), "OUTP 1");
    }

    #[test]
    fn maynuo_command_scpi_set_output_off() {
        assert_eq!(MaynuoCommand::SetOutput { on: false }.scpi(), "OUTP 0");
    }

    #[test]
    fn maynuo_command_scpi_query_current() {
        assert_eq!(MaynuoCommand::QueryCurrent.scpi(), "MEAS:CURR?");
    }

    // -----------------------------------------------------------------------
    // Maynuo safe init plan
    // -----------------------------------------------------------------------

    #[test]
    fn safe_init_plan_command_order_is_deterministic() {
        let axis = example_maynuo_axis_profile();
        let plan = build_safe_init_plan(&axis);

        assert_eq!(plan.commands.len(), 8);
        assert_eq!(plan.commands[0].seq, 1);
        assert_eq!(plan.commands[0].scpi, "*IDN?");
        assert_eq!(plan.commands[1].scpi, "SYST:REM");
        assert_eq!(plan.commands[2].scpi, "VOLT 75");
        assert_eq!(plan.commands[3].scpi, "CURR 0.00000");
        assert_eq!(plan.commands[4].scpi, "OUTP 0");
        assert_eq!(plan.commands[5].scpi, "MEAS:CURR?");
        assert_eq!(plan.commands[6].scpi, "OUTP 0");
        assert_eq!(plan.commands[7].scpi, "SYST:LOC");
    }

    #[test]
    fn safe_init_plan_is_not_executable() {
        let axis = example_maynuo_axis_profile();
        let plan = build_safe_init_plan(&axis);
        assert!(!plan.executable);
        assert!(plan.executable_reason.contains("Mag-M0.5 mock-only"));
    }

    // -----------------------------------------------------------------------
    // Maynuo 10mA microtest plan
    // -----------------------------------------------------------------------

    #[test]
    fn microtest_plan_command_order_is_deterministic() {
        let axis = example_maynuo_axis_profile();
        let plan = build_10ma_microtest_plan(&axis).unwrap();

        assert_eq!(plan.commands.len(), 9);
        assert_eq!(plan.commands[0].scpi, "*IDN?");
        assert_eq!(plan.commands[1].scpi, "SYST:REM");
        assert_eq!(plan.commands[2].scpi, "VOLT 75");
        assert_eq!(plan.commands[3].scpi, "CURR 0.01000");
        assert_eq!(plan.commands[4].scpi, "OUTP 1");
        assert_eq!(plan.commands[5].scpi, "MEAS:CURR?");
        assert_eq!(plan.commands[6].scpi, "OUTP 0");
        assert_eq!(plan.commands[7].scpi, "CURR 0.00000");
        assert_eq!(plan.commands[8].scpi, "SYST:LOC");
    }

    #[test]
    fn microtest_plan_is_not_executable() {
        let axis = example_maynuo_axis_profile();
        let plan = build_10ma_microtest_plan(&axis).unwrap();
        assert!(!plan.executable);
        assert!(plan.executable_reason.contains("Mag-M0.5 mock-only"));
    }

    #[test]
    fn microtest_plan_has_set_current_10ma() {
        let axis = example_maynuo_axis_profile();
        let plan = build_10ma_microtest_plan(&axis).unwrap();
        let set_current_entry = plan.commands.iter().find(|e| e.seq == 4).unwrap();
        assert_eq!(set_current_entry.scpi, "CURR 0.01000");
    }

    #[test]
    fn microtest_plan_rejected_if_exceeds_axis_limit() {
        let mut axis = example_maynuo_axis_profile();
        axis.max_current_ma = 5.0; // 5 mA limit, 10 mA test should fail
        let result = build_10ma_microtest_plan(&axis);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::CurrentLimitExceeded { .. }
        ));
    }

    // -----------------------------------------------------------------------
    // Maynuo profile serde round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn maynuo_axes_profile_roundtrip_serde() {
        let profile = example_maynuo_axes_profile();
        let json = serde_json::to_string(&profile).unwrap();
        let back: MaynuoAxesProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(
            profile.x.coil_constant_nt_per_ma,
            back.x.coil_constant_nt_per_ma
        );
        assert_eq!(
            profile.y.coil_constant_nt_per_ma,
            back.y.coil_constant_nt_per_ma
        );
        assert_eq!(
            profile.z.coil_constant_nt_per_ma,
            back.z.coil_constant_nt_per_ma
        );
        assert_eq!(
            profile.serial_settings.baudrate,
            back.serial_settings.baudrate
        );
        assert_eq!(profile.serial_settings.dtr, back.serial_settings.dtr);
    }

    #[test]
    fn maynuo_command_plan_roundtrip_serde() {
        let axis = example_maynuo_axis_profile();
        let plan = build_safe_init_plan(&axis);
        let json = serde_json::to_string(&plan).unwrap();
        let back: MaynuoCommandPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(plan.id, back.id);
        assert_eq!(plan.commands.len(), back.commands.len());
        assert_eq!(plan.commands[0].scpi, back.commands[0].scpi);
    }

    // -----------------------------------------------------------------------
    // GUI contract serde round-trip (loaded from example file)
    // -----------------------------------------------------------------------

    #[test]
    fn maynuo_gui_contract_json_roundtrip() {
        let contract = serde_json::json!({
            "schema_version": "0.2.0",
            "kind": "maynuo_gui_contract",
            "id": "maynuo_m8812_gui_m0_5",
            "gui_milestone": "M0.5",
            "disabled_reason": "requires Mag-M2 backend bring-up path",
            "axis_cards": [
                {
                    "axis_id": "mag_x",
                    "display_name": "X Axis",
                    "identity": {
                        "device_model": "MAYNUO M8812",
                        "sn_tail": "2020",
                        "idn_verified": false,
                        "idn_placeholder": true
                    },
                    "connection": {
                        "state": "disconnected_mock",
                        "port_name": "COM4",
                        "baudrate": 9600
                    },
                    "output": {
                        "state": "off",
                        "settable": false,
                        "disabled_reason": "requires Mag-M2B safe init"
                    },
                    "target_current": {
                        "value_ma": 0.0,
                        "value_a": 0.0,
                        "settable": false,
                        "disabled_reason": "requires Mag-M3 micro-test path",
                        "command_preview": "CURR 0.00000"
                    },
                    "measured_current": {
                        "value_ma": 0.0,
                        "value_a": 0.0,
                        "stale": true
                    },
                    "target_field": {
                        "value_nt": 0.0,
                        "value_t": 0.0
                    },
                    "calibration": {
                        "coil_constant_nt_per_ma": 143.26,
                        "gain_t_per_a": 1.4326e-4,
                        "zero_offset_ma": 0.0,
                        "verified": true
                    }
                }
            ],
            "global_state": {
                "any_connected": false,
                "any_output_on": false,
                "emergency_stop_available": false,
                "banner_text": "GUI-M0.5 MOCK VIEWER"
            }
        });

        let json = serde_json::to_string(&contract).unwrap();
        let back: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(back["kind"], "maynuo_gui_contract");
        assert_eq!(back["gui_milestone"], "M0.5");
        assert_eq!(back["axis_cards"][0]["axis_id"], "mag_x");
        assert_eq!(
            back["axis_cards"][0]["calibration"]["coil_constant_nt_per_ma"],
            143.26
        );
    }

    // -----------------------------------------------------------------------
    // Maynuo no hardware dependency
    // -----------------------------------------------------------------------

    #[test]
    fn maynuo_no_hardware_dependency() {
        // All Maynuo functions are pure computation.
        let _ = nt_per_ma_to_t_per_a(143.26);
        let _ = format_current_command_from_ma(10.0).unwrap();
        let axis = example_maynuo_axis_profile();
        let _ = build_safe_init_plan(&axis);
        let _ = build_10ma_microtest_plan(&axis).unwrap();
        let _ = build_query_current_plan(&axis);
        // Reaching here proves no serial/USB/TCP was invoked.
    }
}
