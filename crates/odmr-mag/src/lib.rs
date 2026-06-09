//! odmr-mag — magnetic field planning, runtime bridge types, and safety validation.
//!
//! This crate still does not perform serial I/O itself. Real transport remains
//! in `odmr-maynuo-m8812`, while this layer owns typed magnetic-domain models,
//! current planning, zero-lock state, and runtime command/report structures.
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
    /// Invalid state machine transition.
    InvalidStateTransition {
        axis_id: String,
        from: String,
        to: String,
        reason: String,
    },
    /// SN did not match any known axis.
    UnknownSerialNumber { sn: String },
    /// Same SN matched multiple axes.
    DuplicateSerialNumber {
        sn: String,
        axis_a: String,
        axis_b: String,
    },
    /// Required axis not discovered.
    AxisNotDiscovered { axis_id: String },
    /// Lock-zero requested before zero measurement.
    LockZeroBeforeMeasurement { axis_id: String },
    /// Recur setpoint requested before lock-zero.
    RecurBeforeLockZero { axis_id: String },
    /// Total current exceeds hardware max.
    TotalCurrentOverLimit {
        axis_id: String,
        total_ma: f64,
        limit_ma: f64,
    },
    /// Output enabled before safe init.
    OutputBeforeInit { axis_id: String },
    /// Malformed *IDN? response.
    MalformedIdn { idn: String, reason: String },
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
            MagError::InvalidStateTransition {
                axis_id,
                from,
                to,
                reason,
            } => {
                write!(
                    f,
                    "invalid state transition for {axis_id}: {from} -> {to}: {reason}"
                )
            }
            MagError::UnknownSerialNumber { sn } => write!(f, "unknown serial number: {sn}"),
            MagError::DuplicateSerialNumber { sn, axis_a, axis_b } => {
                write!(f, "duplicate SN {sn} matched both {axis_a} and {axis_b}")
            }
            MagError::AxisNotDiscovered { axis_id } => {
                write!(f, "axis {axis_id} was not discovered")
            }
            MagError::LockZeroBeforeMeasurement { axis_id } => {
                write!(
                    f,
                    "lock-zero requested for {axis_id} before zero current measurement"
                )
            }
            MagError::RecurBeforeLockZero { axis_id } => {
                write!(
                    f,
                    "recurrent setpoint requested for {axis_id} before lock-zero"
                )
            }
            MagError::TotalCurrentOverLimit {
                axis_id,
                total_ma,
                limit_ma,
            } => {
                write!(
                    f,
                    "total current {total_ma:.3} mA exceeds limit {limit_ma:.3} mA on {axis_id}"
                )
            }
            MagError::OutputBeforeInit { axis_id } => {
                write!(f, "output enabled on {axis_id} before safe init")
            }
            MagError::MalformedIdn { idn, reason } => {
                write!(f, "malformed *IDN? response: {reason}: {idn}")
            }
        }
    }
}

impl std::error::Error for MagError {}

// ---------------------------------------------------------------------------
// Runtime bridge types
// ---------------------------------------------------------------------------

/// Typed runtime command emitted by higher-level executors for a magnetic axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MagRuntimeCommand {
    ApplyCurrent {
        axis_id: String,
        current_a: f64,
        enable_output: bool,
    },
    QueryReadback {
        axis_id: String,
    },
    LockZero {
        axis_id: String,
        zero_current_a: f64,
    },
    CleanupAxis {
        axis_id: String,
        verify_current_decay: bool,
    },
}

/// Runtime readback from a magnetic axis after an apply or cleanup stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagReadback {
    pub axis_id: String,
    pub measured_current_a: f64,
    pub output_enabled: bool,
    pub zero_locked: bool,
}

/// Cleanup result for one or more axes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagCleanupReport {
    pub axes: Vec<MagReadback>,
    pub all_outputs_disabled: bool,
    pub max_residual_current_a: f64,
}

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

/// Metadata for a device fingerprint verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationMetadata {
    pub method: String,
    pub date: String,
    pub verified_by: String,
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Three-axis wrapper for Maynuo M8812 axis profiles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaynuoAxes {
    pub x: MaynuoAxisProfile,
    pub y: MaynuoAxisProfile,
    pub z: MaynuoAxisProfile,
}

/// Per-axis profile for a Maynuo M8812 magnetic axis.
///
/// `last_known_port_name` is a hint for the operator; it MUST NOT be used as a
/// stable device identity.  Only `expected_idn` (or the SN tail within it) is
/// a reliable binding key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaynuoAxisProfile {
    pub axis_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub display_name: Option<String>,
    /// Hint only — not a stable identity.  Port paths are dynamic per session.
    #[serde(alias = "port_name")]
    pub last_known_port_name: String,
    pub device_model: String,
    pub sn_tail: String,
    /// Stable binding key.  Contains full SN: `MAYNUO,M8812,<SN>,V2.7`
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
    /// Hardware max current in mA (M8812 spec = 2000, not M8811's 5000)
    pub max_current_ma: f64,
    /// Hardware max current in A (M8812 spec = 2.0)
    pub max_current_a: f64,
    pub voltage_v: u16,
}

/// Three-axis assembly profile for the lab Maynuo M8812 setup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaynuoAxesProfile {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub serial_settings: MaynuoSerialSettings,
    pub axes: MaynuoAxes,
    pub safety_policy_id: String,
    pub calibration_date: String,
    pub verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification: Option<VerificationMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl MaynuoAxesProfile {
    /// Build a diagonal CoilMatrix from the per-axis coil constants.
    ///
    /// Returns an error if any gain is non-finite, <= 0, or if any zero offset
    /// is non-finite.  This bridges the Maynuo axis-gain model to the general
    /// 3×3 matrix model.
    pub fn try_to_coil_matrix(&self) -> Result<CoilMatrix, MagError> {
        let kx = self.axes.x.gain_t_per_a;
        let ky = self.axes.y.gain_t_per_a;
        let kz = self.axes.z.gain_t_per_a;

        // Reject non-finite gains
        for (axis, gain) in [('x', kx), ('y', ky), ('z', kz)] {
            if !gain.is_finite() {
                return Err(MagError::NonFiniteValue {
                    field: format!("{axis}_gain_t_per_a"),
                    value: gain,
                });
            }
            if gain <= 0.0 {
                return Err(MagError::CalibrationMissing {
                    field: format!("{axis}_gain_t_per_a is non-positive: {gain}"),
                });
            }
        }

        // Reject non-finite zero offsets
        let offsets = [
            ('x', self.axes.x.zero_offset_a),
            ('y', self.axes.y.zero_offset_a),
            ('z', self.axes.z.zero_offset_a),
        ];
        for (axis, offset) in offsets {
            if !offset.is_finite() {
                return Err(MagError::NonFiniteValue {
                    field: format!("{axis}_zero_offset_a"),
                    value: offset,
                });
            }
        }

        let abs_gains = [kx.abs(), ky.abs(), kz.abs()];
        let max_gain = abs_gains.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min_gain = abs_gains.iter().copied().fold(f64::INFINITY, f64::min);
        let condition_number = if min_gain > 0.0 {
            max_gain / min_gain
        } else {
            f64::INFINITY
        };

        Ok(CoilMatrix {
            m: [[kx, 0.0, 0.0], [0.0, ky, 0.0], [0.0, 0.0, kz]],
            i_offset_a: [
                self.axes.x.zero_offset_a,
                self.axes.y.zero_offset_a,
                self.axes.z.zero_offset_a,
            ],
            b_zero_offset_t: [0.0, 0.0, 0.0],
            condition_number: Some(condition_number),
            calibrated_at: self.calibration_date.clone(),
            verified: self.verified,
            verified_by: self.verified_by.clone(),
        })
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
    SetVoltageProtection { voltage_v: u16 },
    SetCurrent { current_a: f64, current_ma: f64 },
    SetOutput { on: bool },
    QueryCurrent,
    QueryError,
}

impl MaynuoCommand {
    /// Return the SCPI string for this command.
    pub fn scpi(&self) -> String {
        match self {
            MaynuoCommand::Identify => "*IDN?".into(),
            MaynuoCommand::SetRemote => "SYST:REM".into(),
            MaynuoCommand::SetLocal => "SYST:LOC".into(),
            MaynuoCommand::SetVoltage { voltage_v } => format!("VOLT {voltage_v}"),
            MaynuoCommand::SetVoltageProtection { voltage_v } => {
                format!("VOLT:PROT {voltage_v}")
            }
            MaynuoCommand::SetCurrent { current_a, .. } => {
                format!("CURR {current_a:.5}")
            }
            MaynuoCommand::SetOutput { on } => {
                format!("OUTP {}", if *on { 1 } else { 0 })
            }
            MaynuoCommand::QueryCurrent => "MEAS:CURR?".into(),
            MaynuoCommand::QueryError => "SYST:ERR?".into(),
        }
    }

    /// Whether this command expects a response from the device.
    ///
    /// Per reverse analysis (verify_protocol.py): only `*IDN?` and `MEAS:CURR?`
    /// return data.  Set commands are fire-and-forget — the M8812 does not
    /// acknowledge them.
    ///
    /// `SYST:ERR?` is documented in the M8812 manual and returns error codes.
    pub fn expects_response(&self) -> bool {
        matches!(
            self,
            MaynuoCommand::Identify | MaynuoCommand::QueryCurrent | MaynuoCommand::QueryError
        )
    }

    /// Return the expected response shape (for documentation / validation).
    /// Returns `"none"` for set commands that do not receive a response.
    pub fn expected_response_shape(&self) -> &'static str {
        match self {
            MaynuoCommand::Identify => "MAYNUO,M8812,<SN>,V2.7",
            MaynuoCommand::SetRemote => "none",
            MaynuoCommand::SetLocal => "none",
            MaynuoCommand::SetVoltage { .. } => "none",
            MaynuoCommand::SetVoltageProtection { .. } => "none",
            MaynuoCommand::SetCurrent { .. } => "none",
            MaynuoCommand::SetOutput { .. } => "none",
            MaynuoCommand::QueryCurrent => "float_ampere",
            MaynuoCommand::QueryError => "error_code,message",
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub axis_id: String,
    pub profile_id: String,
    pub executable: bool,
    pub executable_reason: String,
    /// `"verified_normal"` | `"emergency"` | `null` (non-shutdown plans)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shutdown_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_duration_ms: Option<u64>,
    pub commands: Vec<MaynuoCommandEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaynuoCommandEntry {
    pub seq: u32,
    pub command_type: String,
    pub scpi: String,
    /// Whether the device actually returns data for this command.
    /// Per verify_protocol.py: only `*IDN?` and `MEAS:CURR?` return responses.
    /// Set commands (SYST:REM, VOLT, CURR, OUTP, SYST:LOC) do not.
    pub expects_response: bool,
    /// Expected response shape, or `"none"` if `expects_response` is `false`.
    pub expected_response_shape: String,
    pub event_name: String,
    pub blocking: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

#[allow(clippy::too_many_arguments)]
fn cmd_entry(
    seq: u32,
    command_type: &str,
    scpi: &str,
    expects_response: bool,
    expected_response_shape: &str,
    event_name: &str,
    blocking: bool,
    delay_ms: Option<u64>,
    timeout_ms: Option<u64>,
) -> MaynuoCommandEntry {
    MaynuoCommandEntry {
        seq,
        command_type: command_type.into(),
        scpi: scpi.into(),
        expects_response,
        expected_response_shape: expected_response_shape.into(),
        event_name: event_name.into(),
        blocking,
        delay_ms,
        timeout_ms,
    }
}

/// Build a safe initialization command plan for a single axis.
pub fn build_safe_init_plan(axis: &MaynuoAxisProfile) -> MaynuoCommandPlan {
    let entries = vec![
        cmd_entry(
            1,
            "identify",
            "*IDN?",
            true,
            "MAYNUO,M8812,<SN>,V2.7",
            "mag_idn_queried",
            true,
            None,
            Some(300),
        ),
        cmd_entry(
            2,
            "set_remote",
            "SYST:REM",
            false,
            "none",
            "mag_remote_mode_set",
            false,
            Some(50),
            None,
        ),
        cmd_entry(
            3,
            "set_voltage",
            &format!("VOLT {}", axis.voltage_v),
            false,
            "none",
            "mag_voltage_set",
            false,
            Some(50),
            None,
        ),
        cmd_entry(
            4,
            "set_voltage_protection",
            &format!("VOLT:PROT {}", axis.voltage_v),
            false,
            "none",
            "mag_voltage_protection_set",
            false,
            Some(50),
            None,
        ),
        cmd_entry(
            5,
            "set_current",
            "CURR 0.00000",
            false,
            "none",
            "mag_current_zeroed",
            false,
            Some(50),
            None,
        ),
        cmd_entry(
            6,
            "set_output",
            "OUTP 0",
            false,
            "none",
            "mag_output_disabled",
            false,
            Some(50),
            None,
        ),
        cmd_entry(
            7,
            "query_current",
            "MEAS:CURR?",
            true,
            "float_ampere",
            "mag_current_queried",
            true,
            None,
            Some(300),
        ),
        cmd_entry(
            7,
            "set_output",
            "OUTP 0",
            false,
            "none",
            "mag_output_confirmed_off",
            false,
            Some(50),
            None,
        ),
        cmd_entry(
            8,
            "set_local",
            "SYST:LOC",
            false,
            "none",
            "mag_local_mode_set",
            false,
            Some(50),
            None,
        ),
    ];

    MaynuoCommandPlan {
        schema_version: "0.2.0".into(),
        kind: "maynuo_command_plan".into(),
        id: format!("maynuo_safe_init_{}", axis.axis_id),
        name: "Maynuo M8812 Safe Initialization Plan".into(),
        description: Some("Dry-run command plan for safe initialization of a single Maynuo M8812 axis. No hardware I/O is performed.".into()),
        axis_id: axis.axis_id.clone(),
        profile_id: "maynuo_m8812_lab_xyz".into(),
        executable: false,
        executable_reason:
            "Mag-M0.5 mock-only: requires Mag-M2B backend bring-up for real execution".into(),
        shutdown_mode: None,
        expected_duration_ms: Some(500),
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
        description: None,
        axis_id: axis.axis_id.clone(),
        profile_id: "maynuo_m8812_lab_xyz".into(),
        executable: false,
        executable_reason: "Mag-M0.5 mock-only: requires Mag-M2 backend for real execution".into(),
        shutdown_mode: None,
        expected_duration_ms: Some(300),
        commands: vec![cmd_entry(
            1,
            "query_current",
            "MEAS:CURR?",
            true,
            "float_ampere",
            "mag_current_queried",
            true,
            None,
            Some(300),
        )],
    }
}

/// Build a 10 mA micro-test command plan for a single axis.
///
/// Uses verified normal shutdown: CURR 0 → OUTP 0 → SYST:LOC.
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
        cmd_entry(
            1,
            "identify",
            "*IDN?",
            true,
            "MAYNUO,M8812,<SN>,V2.7",
            "mag_idn_queried",
            true,
            None,
            Some(300),
        ),
        cmd_entry(
            2,
            "set_remote",
            "SYST:REM",
            false,
            "none",
            "mag_remote_mode_set",
            false,
            Some(50),
            None,
        ),
        cmd_entry(
            3,
            "set_voltage",
            &format!("VOLT {}", axis.voltage_v),
            false,
            "none",
            "mag_voltage_set",
            false,
            Some(50),
            None,
        ),
        cmd_entry(
            4,
            "set_current",
            &format!("CURR {test_current_a:.5}"),
            false,
            "none",
            "mag_current_set",
            false,
            Some(50),
            None,
        ),
        cmd_entry(
            5,
            "set_output",
            "OUTP 1",
            false,
            "none",
            "mag_output_enabled",
            false,
            Some(200),
            None,
        ),
        cmd_entry(
            6,
            "query_current",
            "MEAS:CURR?",
            true,
            "float_ampere",
            "mag_current_measured",
            true,
            None,
            Some(300),
        ),
        // Verified normal shutdown: CURR 0 → OUTP 0 → SYST:LOC
        cmd_entry(
            7,
            "set_current",
            "CURR 0.00000",
            false,
            "none",
            "mag_current_zeroed",
            false,
            Some(50),
            None,
        ),
        cmd_entry(
            8,
            "set_output",
            "OUTP 0",
            false,
            "none",
            "mag_output_disabled",
            false,
            Some(50),
            None,
        ),
        cmd_entry(
            9,
            "set_local",
            "SYST:LOC",
            false,
            "none",
            "mag_local_mode_set",
            false,
            Some(50),
            None,
        ),
    ];

    Ok(MaynuoCommandPlan {
        schema_version: "0.2.0".into(),
        kind: "maynuo_command_plan".into(),
        id: format!("maynuo_10ma_microtest_{}", axis.axis_id),
        name: "Maynuo M8812 10 mA Micro-Test Plan".into(),
        description: Some("Future Mag-M3 candidate: low-current single-axis micro-test with verified normal shutdown.".into()),
        axis_id: axis.axis_id.clone(),
        profile_id: "maynuo_m8812_lab_xyz".into(),
        executable: false,
        executable_reason:
            "Mag-M0.5 mock-only: future Mag-M3 candidate, not executable in Mag-M0.5".into(),
        shutdown_mode: Some("verified_normal".into()),
        expected_duration_ms: Some(1500),
        commands: entries,
    })
}

/// Build a verified-normal shutdown plan for a single axis.
///
/// Sequence: CURR 0 → OUTP 0 → SYST:LOC
/// Matches the disconnect sequence from verify_protocol.py and the original
/// disconnection sequence in FormMain.cs.
pub fn build_verified_normal_shutdown_plan(axis: &MaynuoAxisProfile) -> MaynuoCommandPlan {
    MaynuoCommandPlan {
        schema_version: "0.2.0".into(),
        kind: "maynuo_command_plan".into(),
        id: format!("maynuo_normal_shutdown_{}", axis.axis_id),
        name: "Maynuo M8812 Verified Normal Shutdown Plan".into(),
        description: Some(
            "Disconnect sequence confirmed by verify_protocol.py: CURR 0 → OUTP 0 → SYST:LOC"
                .into(),
        ),
        axis_id: axis.axis_id.clone(),
        profile_id: "maynuo_m8812_lab_xyz".into(),
        executable: false,
        executable_reason: "Mag-M0.5 mock-only: requires Mag-M2B backend".into(),
        shutdown_mode: Some("verified_normal".into()),
        expected_duration_ms: Some(150),
        commands: vec![
            cmd_entry(
                1,
                "set_current",
                "CURR 0.00000",
                false,
                "none",
                "mag_current_zeroed",
                false,
                Some(50),
                None,
            ),
            cmd_entry(
                2,
                "set_output",
                "OUTP 0",
                false,
                "none",
                "mag_output_disabled",
                false,
                Some(50),
                None,
            ),
            cmd_entry(
                3,
                "set_local",
                "SYST:LOC",
                false,
                "none",
                "mag_local_mode_set",
                false,
                Some(50),
                None,
            ),
        ],
    }
}

/// Build an emergency shutdown plan for a single axis.
///
/// Emergency sequence: OUTP 0 → CURR 0 → SYST:LOC
/// Output is killed first for maximum speed; current is zeroed second.
pub fn build_emergency_shutdown_plan(axis: &MaynuoAxisProfile) -> MaynuoCommandPlan {
    MaynuoCommandPlan {
        schema_version: "0.2.0".into(),
        kind: "maynuo_command_plan".into(),
        id: format!("maynuo_emergency_shutdown_{}", axis.axis_id),
        name: "Maynuo M8812 Emergency Shutdown Plan".into(),
        description: Some(
            "Emergency sequence: OUTP 0 first for maximum safety, then CURR 0 and SYST:LOC".into(),
        ),
        axis_id: axis.axis_id.clone(),
        profile_id: "maynuo_m8812_lab_xyz".into(),
        executable: false,
        executable_reason: "Mag-M0.5 mock-only: requires Mag-M2B backend with emergency authority"
            .into(),
        shutdown_mode: Some("emergency".into()),
        expected_duration_ms: Some(150),
        commands: vec![
            cmd_entry(
                1,
                "set_output",
                "OUTP 0",
                false,
                "none",
                "mag_output_disabled_emergency",
                false,
                Some(0),
                None,
            ),
            cmd_entry(
                2,
                "set_current",
                "CURR 0.00000",
                false,
                "none",
                "mag_current_zeroed_emergency",
                false,
                Some(50),
                None,
            ),
            cmd_entry(
                3,
                "set_local",
                "SYST:LOC",
                false,
                "none",
                "mag_local_mode_set",
                false,
                Some(50),
                None,
            ),
        ],
    }
}

// ---------------------------------------------------------------------------
// Maynuo axis state machine (Mag-M1)
// ---------------------------------------------------------------------------

/// The operational state of a single Maynuo M8812 magnetic axis.
///
/// All transitions are deterministic and mock-only.  No hardware I/O.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MaynuoAxisState {
    Unknown,
    Discovered {
        idn: String,
    },
    AxisMapped {
        axis_id: String,
        idn: String,
    },
    InitializedOutputOff {
        axis_id: String,
        idn: String,
    },
    OutputOnZeroMode {
        axis_id: String,
        idn: String,
    },
    ZeroMeasured {
        axis_id: String,
        idn: String,
        zero_current_ma: f64,
    },
    ZeroLocked {
        axis_id: String,
        idn: String,
        zero_current_ma: f64,
    },
    RecurSetpointPlanned {
        axis_id: String,
        idn: String,
        zero_current_ma: f64,
        recur_current_ma: f64,
        recur_field_nt: f64,
    },
    RecurSetpointAppliedMock {
        axis_id: String,
        idn: String,
        zero_current_ma: f64,
        recur_current_ma: f64,
        recur_field_nt: f64,
        total_current_ma: f64,
    },
    ShutdownNormal {
        axis_id: String,
    },
    ShutdownEmergency {
        axis_id: String,
    },
    Fault {
        axis_id: String,
        reason: String,
    },
}

impl MaynuoAxisState {
    pub fn state_name(&self) -> &'static str {
        match self {
            MaynuoAxisState::Unknown => "Unknown",
            MaynuoAxisState::Discovered { .. } => "Discovered",
            MaynuoAxisState::AxisMapped { .. } => "AxisMapped",
            MaynuoAxisState::InitializedOutputOff { .. } => "InitializedOutputOff",
            MaynuoAxisState::OutputOnZeroMode { .. } => "OutputOnZeroMode",
            MaynuoAxisState::ZeroMeasured { .. } => "ZeroMeasured",
            MaynuoAxisState::ZeroLocked { .. } => "ZeroLocked",
            MaynuoAxisState::RecurSetpointPlanned { .. } => "RecurSetpointPlanned",
            MaynuoAxisState::RecurSetpointAppliedMock { .. } => "RecurSetpointAppliedMock",
            MaynuoAxisState::ShutdownNormal { .. } => "ShutdownNormal",
            MaynuoAxisState::ShutdownEmergency { .. } => "ShutdownEmergency",
            MaynuoAxisState::Fault { .. } => "Fault",
        }
    }

    pub fn is_safe(&self) -> bool {
        matches!(
            self,
            MaynuoAxisState::ShutdownNormal { .. }
                | MaynuoAxisState::ShutdownEmergency { .. }
                | MaynuoAxisState::Unknown
                | MaynuoAxisState::InitializedOutputOff { .. }
        )
    }

    pub fn axis_id(&self) -> Option<&str> {
        match self {
            MaynuoAxisState::Unknown | MaynuoAxisState::Discovered { .. } => None,
            MaynuoAxisState::AxisMapped { axis_id, .. }
            | MaynuoAxisState::InitializedOutputOff { axis_id, .. }
            | MaynuoAxisState::OutputOnZeroMode { axis_id, .. }
            | MaynuoAxisState::ZeroMeasured { axis_id, .. }
            | MaynuoAxisState::ZeroLocked { axis_id, .. }
            | MaynuoAxisState::RecurSetpointPlanned { axis_id, .. }
            | MaynuoAxisState::RecurSetpointAppliedMock { axis_id, .. }
            | MaynuoAxisState::ShutdownNormal { axis_id }
            | MaynuoAxisState::ShutdownEmergency { axis_id }
            | MaynuoAxisState::Fault { axis_id, .. } => Some(axis_id),
        }
    }
}

/// A record in the axis state timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaynuoAxisStateEvent {
    pub seq: u64,
    pub timestamp: String,
    pub axis_id: String,
    pub from_state: String,
    pub to_state: String,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

// ---------------------------------------------------------------------------
// IDN parsing and SN-based discovery matching (Mag-M1.1)
// ---------------------------------------------------------------------------

/// Parsed *IDN? response from a Maynuo M8812.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaynuoIdn {
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    pub firmware: Option<String>,
}

/// Parse an *IDN? response string into a structured MaynuoIdn.
///
/// Expected format: `MAYNUO,M8812,<SN>,<FW>`
/// Requires manufacturer contains MAYNUO, model == M8812, non-empty SN.
pub fn parse_maynuo_idn(idn: &str) -> Result<MaynuoIdn, MagError> {
    let reject = |reason: String| {
        Err::<MaynuoIdn, MagError>(MagError::MalformedIdn {
            idn: idn.into(),
            reason,
        })
    };

    if idn.is_empty() {
        return reject("empty response".to_string());
    }
    let fields: Vec<&str> = idn.split(',').collect();
    if fields.len() < 3 {
        return reject(format!("expected >=3 comma fields, got {}", fields.len()));
    }
    let manufacturer = fields[0].trim().to_string();
    let model = fields[1].trim().to_string();
    let serial_number = fields[2].trim().to_string();
    let firmware = fields.get(3).map(|s| s.trim().to_string());

    if !manufacturer.to_uppercase().contains("MAYNUO") {
        return reject(format!("not a Maynuo device: manufacturer={manufacturer}"));
    }
    if model != "M8812" {
        return reject(format!("not an M8812: model={model}"));
    }
    if serial_number.is_empty() {
        return reject("empty serial number field".to_string());
    }
    Ok(MaynuoIdn {
        manufacturer,
        model,
        serial_number,
        firmware,
    })
}

/// Extract the expected serial number from an expected_idn string.
///
/// Parses the expected_idn and returns just the serial number (third field).
pub fn expected_sn_from_idn(expected_idn: &str) -> Result<String, MagError> {
    parse_maynuo_idn(expected_idn).map(|p| p.serial_number)
}

/// Match observed *IDN? responses to logical axes by exact SN equality.
///
/// Rejects: unknown SN, malformed IDN, empty SN, duplicate observed SN,
/// duplicate mapping to same axis, missing required axes.
pub fn match_axes_by_idn(
    profile: &MaynuoAxesProfile,
    observed_idns: &[String],
) -> Result<std::collections::BTreeMap<String, String>, MagError> {
    let mut matched = std::collections::BTreeMap::new();
    let mut seen_sn: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();

    for idn in observed_idns {
        let parsed = parse_maynuo_idn(idn)?;
        let sn = &parsed.serial_number;

        // Reject duplicate observed SN
        if let Some(prev_idn) = seen_sn.get(sn) {
            return Err(MagError::DuplicateSerialNumber {
                sn: sn.clone(),
                axis_a: prev_idn.clone(),
                axis_b: "(duplicate observed)".into(),
            });
        }
        seen_sn.insert(sn.clone(), idn.clone());

        // Find axis whose expected_idn contains exactly this SN
        let candidates: Vec<&MaynuoAxisProfile> =
            [&profile.axes.x, &profile.axes.y, &profile.axes.z]
                .iter()
                .filter(|a| {
                    expected_sn_from_idn(&a.expected_idn)
                        .map(|esn| &esn == sn)
                        .unwrap_or(false)
                })
                .copied()
                .collect();

        if candidates.is_empty() {
            return Err(MagError::UnknownSerialNumber { sn: sn.clone() });
        }
        if candidates.len() > 1 {
            return Err(MagError::DuplicateSerialNumber {
                sn: sn.clone(),
                axis_a: candidates[0].axis_id.clone(),
                axis_b: candidates[1].axis_id.clone(),
            });
        }
        let axis_id = &candidates[0].axis_id;
        if matched.contains_key(axis_id) {
            return Err(MagError::DuplicateSerialNumber {
                sn: sn.clone(),
                axis_a: axis_id.clone(),
                axis_b: axis_id.clone(),
            });
        }
        matched.insert(axis_id.clone(), idn.clone());
    }
    for axis_id in ["mag_x", "mag_y", "mag_z"] {
        if !matched.contains_key(axis_id) {
            return Err(MagError::AxisNotDiscovered {
                axis_id: axis_id.into(),
            });
        }
    }
    Ok(matched)
}

// ---------------------------------------------------------------------------
// Maynuo axis runner (Mag-M1)
// ---------------------------------------------------------------------------

/// In-memory state machine runner for a single Maynuo magnetic axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaynuoAxisRunner {
    pub axis_id: String,
    pub state: MaynuoAxisState,
    pub profile: MaynuoAxisProfile,
    pub zero_current_ma: Option<f64>,
    pub recur_current_ma: Option<f64>,
    pub recur_field_nt: Option<f64>,
    pub total_current_ma: Option<f64>,
    pub measured_total_current_ma: Option<f64>,
    pub lock_zero: bool,
    pub output: bool,
    pub seq: u64,
    pub events: Vec<MaynuoAxisStateEvent>,
}

impl MaynuoAxisRunner {
    pub fn new(profile: MaynuoAxisProfile) -> Self {
        Self {
            axis_id: profile.axis_id.clone(),
            state: MaynuoAxisState::Unknown,
            profile,
            zero_current_ma: None,
            recur_current_ma: None,
            recur_field_nt: None,
            total_current_ma: None,
            measured_total_current_ma: None,
            lock_zero: false,
            output: false,
            seq: 0,
            events: Vec::new(),
        }
    }

    fn transition_to(
        &mut self,
        to: MaynuoAxisState,
        event_type: &str,
        detail: Option<String>,
    ) -> Result<(), MagError> {
        let from_name = self.state.state_name().to_string();
        let to_name = to.state_name().to_string();
        self.state = to;
        self.seq += 1;
        self.events.push(MaynuoAxisStateEvent {
            seq: self.seq,
            timestamp: "2026-06-01T00:00:00Z".into(),
            axis_id: self.axis_id.clone(),
            from_state: from_name,
            to_state: to_name,
            event_type: event_type.into(),
            detail,
        });
        Ok(())
    }

    pub fn apply_discovered(&mut self, idn: &str) -> Result<(), MagError> {
        if !matches!(self.state, MaynuoAxisState::Unknown) {
            return Err(MagError::InvalidStateTransition {
                axis_id: self.axis_id.clone(),
                from: self.state.state_name().into(),
                to: "Discovered".into(),
                reason: "already discovered".into(),
            });
        }
        let parsed = parse_maynuo_idn(idn)?;
        let expected_sn = expected_sn_from_idn(&self.profile.expected_idn)?;
        if parsed.serial_number != expected_sn {
            return Err(MagError::UnknownSerialNumber {
                sn: parsed.serial_number,
            });
        }
        self.transition_to(
            MaynuoAxisState::Discovered { idn: idn.into() },
            "mag_axis_discovered",
            Some(format!("idn={idn}")),
        )
    }

    pub fn apply_axis_mapped(&mut self) -> Result<(), MagError> {
        let idn = match &self.state {
            MaynuoAxisState::Discovered { idn } => idn.clone(),
            _ => {
                return Err(MagError::InvalidStateTransition {
                    axis_id: self.axis_id.clone(),
                    from: self.state.state_name().into(),
                    to: "AxisMapped".into(),
                    reason: "must be Discovered first".into(),
                })
            }
        };
        self.transition_to(
            MaynuoAxisState::AxisMapped {
                axis_id: self.axis_id.clone(),
                idn,
            },
            "mag_axis_mapped",
            Some(format!("axis_id={}", self.axis_id)),
        )
    }

    pub fn apply_initialized_output_off(&mut self) -> Result<(), MagError> {
        let (my_id, idn) = match &self.state {
            MaynuoAxisState::AxisMapped { axis_id, idn } => (axis_id.clone(), idn.clone()),
            MaynuoAxisState::InitializedOutputOff { axis_id, idn, .. } => {
                (axis_id.clone(), idn.clone())
            }
            _ => {
                return Err(MagError::InvalidStateTransition {
                    axis_id: self.axis_id.clone(),
                    from: self.state.state_name().into(),
                    to: "InitializedOutputOff".into(),
                    reason: "must be AxisMapped first".into(),
                })
            }
        };
        if my_id != self.axis_id {
            return Err(MagError::InvalidStateTransition {
                axis_id: self.axis_id.clone(),
                from: self.state.state_name().into(),
                to: "InitializedOutputOff".into(),
                reason: "axis_id mismatch".into(),
            });
        }
        self.transition_to(
            MaynuoAxisState::InitializedOutputOff {
                axis_id: self.axis_id.clone(),
                idn,
            },
            "mag_init_complete",
            None,
        )
    }

    pub fn apply_output_on_zero_mode(&mut self) -> Result<(), MagError> {
        let idn = match &self.state {
            MaynuoAxisState::InitializedOutputOff { axis_id, idn } if axis_id == &self.axis_id => {
                idn.clone()
            }
            _ => {
                return Err(MagError::OutputBeforeInit {
                    axis_id: self.axis_id.clone(),
                })
            }
        };
        self.output = true;
        self.transition_to(
            MaynuoAxisState::OutputOnZeroMode {
                axis_id: self.axis_id.clone(),
                idn,
            },
            "mag_output_on_zero",
            None,
        )
    }

    pub fn apply_zero_measured(&mut self, measured_ma: f64) -> Result<(), MagError> {
        if !measured_ma.is_finite() || measured_ma < 0.0 {
            return Err(MagError::NonFiniteValue {
                field: "zero_current_ma".into(),
                value: measured_ma,
            });
        }
        let idn = match &self.state {
            MaynuoAxisState::OutputOnZeroMode { axis_id, idn } if axis_id == &self.axis_id => {
                idn.clone()
            }
            _ => {
                return Err(MagError::InvalidStateTransition {
                    axis_id: self.axis_id.clone(),
                    from: self.state.state_name().into(),
                    to: "ZeroMeasured".into(),
                    reason: "must be OutputOnZeroMode first".into(),
                })
            }
        };
        self.zero_current_ma = Some(measured_ma);
        self.measured_total_current_ma = Some(measured_ma);
        self.transition_to(
            MaynuoAxisState::ZeroMeasured {
                axis_id: self.axis_id.clone(),
                idn,
                zero_current_ma: measured_ma,
            },
            "mag_zero_measured",
            Some(format!("zero={measured_ma:.3} mA")),
        )
    }

    pub fn apply_lock_zero(&mut self) -> Result<(), MagError> {
        let (idn, zero_ma) = match &self.state {
            MaynuoAxisState::ZeroMeasured {
                axis_id,
                idn,
                zero_current_ma,
            }
            | MaynuoAxisState::ZeroLocked {
                axis_id,
                idn,
                zero_current_ma,
            } if axis_id == &self.axis_id => (idn.clone(), *zero_current_ma),
            _ => {
                return Err(MagError::LockZeroBeforeMeasurement {
                    axis_id: self.axis_id.clone(),
                })
            }
        };
        self.lock_zero = true;
        self.zero_current_ma = Some(zero_ma);
        self.transition_to(
            MaynuoAxisState::ZeroLocked {
                axis_id: self.axis_id.clone(),
                idn,
                zero_current_ma: zero_ma,
            },
            "mag_lock_zero_enabled",
            Some(format!("zero={zero_ma:.3} mA")),
        )
    }

    pub fn apply_recur_setpoint_planned_from_field(
        &mut self,
        target_field_nt: f64,
    ) -> Result<(), MagError> {
        let (idn, zero_ma) = match &self.state {
            MaynuoAxisState::ZeroLocked {
                axis_id,
                idn,
                zero_current_ma,
            }
            | MaynuoAxisState::RecurSetpointPlanned {
                axis_id,
                idn,
                zero_current_ma,
                ..
            }
            | MaynuoAxisState::RecurSetpointAppliedMock {
                axis_id,
                idn,
                zero_current_ma,
                ..
            } if axis_id == &self.axis_id => (idn.clone(), *zero_current_ma),
            _ => {
                return Err(MagError::RecurBeforeLockZero {
                    axis_id: self.axis_id.clone(),
                })
            }
        };
        if !target_field_nt.is_finite() {
            return Err(MagError::NonFiniteValue {
                field: "target_field_nt".into(),
                value: target_field_nt,
            });
        }
        let recur_ma = nt_to_ma(target_field_nt, self.profile.coil_constant_nt_per_ma);
        let total_ma = zero_ma + recur_ma;
        if total_ma < 0.0 {
            return Err(MagError::SafetyViolation {
                message: format!(
                    "negative total current {total_ma:.5} mA on {}",
                    self.axis_id
                ),
            });
        }
        if total_ma > self.profile.max_current_ma {
            return Err(MagError::TotalCurrentOverLimit {
                axis_id: self.axis_id.clone(),
                total_ma,
                limit_ma: self.profile.max_current_ma,
            });
        }
        self.recur_current_ma = Some(recur_ma);
        self.recur_field_nt = Some(target_field_nt);
        self.total_current_ma = Some(total_ma);
        self.transition_to(
            MaynuoAxisState::RecurSetpointPlanned {
                axis_id: self.axis_id.clone(),
                idn,
                zero_current_ma: zero_ma,
                recur_current_ma: recur_ma,
                recur_field_nt: target_field_nt,
            },
            "mag_recur_setpoint_planned",
            Some(format!(
                "field={target_field_nt:.2} nT, recur={recur_ma:.5} mA, total={total_ma:.5} mA"
            )),
        )
    }

    pub fn apply_recur_setpoint_applied_mock(&mut self) -> Result<(), MagError> {
        let (idn, zero_ma, recur_ma, recur_field, total_ma) = match &self.state {
            MaynuoAxisState::RecurSetpointPlanned {
                axis_id,
                idn,
                zero_current_ma,
                recur_current_ma,
                recur_field_nt,
            } if axis_id == &self.axis_id => (
                idn.clone(),
                *zero_current_ma,
                *recur_current_ma,
                *recur_field_nt,
                zero_current_ma + recur_current_ma,
            ),
            _ => {
                return Err(MagError::InvalidStateTransition {
                    axis_id: self.axis_id.clone(),
                    from: self.state.state_name().into(),
                    to: "RecurSetpointAppliedMock".into(),
                    reason: "must be RecurSetpointPlanned first".into(),
                })
            }
        };
        self.total_current_ma = Some(total_ma);
        let cmd = format_current_command_from_ma(total_ma)?;
        self.transition_to(
            MaynuoAxisState::RecurSetpointAppliedMock {
                axis_id: self.axis_id.clone(),
                idn,
                zero_current_ma: zero_ma,
                recur_current_ma: recur_ma,
                recur_field_nt: recur_field,
                total_current_ma: total_ma,
            },
            "mag_recur_setpoint_applied",
            Some(format!("cmd={cmd}, total={total_ma:.5} mA")),
        )
    }

    pub fn apply_shutdown_normal(&mut self) -> Result<(), MagError> {
        self.output = false;
        self.transition_to(
            MaynuoAxisState::ShutdownNormal {
                axis_id: self.axis_id.clone(),
            },
            "mag_shutdown_normal",
            None,
        )
    }

    pub fn apply_shutdown_emergency(&mut self) -> Result<(), MagError> {
        self.output = false;
        self.transition_to(
            MaynuoAxisState::ShutdownEmergency {
                axis_id: self.axis_id.clone(),
            },
            "mag_shutdown_emergency",
            None,
        )
    }

    // ---- Readback ----

    pub fn readback_recur_current_ma(&self, measured_total_ma: f64) -> Result<f64, MagError> {
        if !measured_total_ma.is_finite() || measured_total_ma < 0.0 {
            return Err(MagError::NonFiniteValue {
                field: "measured_total_ma".into(),
                value: measured_total_ma,
            });
        }
        if self.lock_zero {
            let zero = self
                .zero_current_ma
                .ok_or(MagError::LockZeroBeforeMeasurement {
                    axis_id: self.axis_id.clone(),
                })?;
            Ok(measured_total_ma - zero)
        } else {
            Ok(0.0)
        }
    }

    pub fn readback_recur_field_nt(&self, recur_current_ma: f64) -> f64 {
        ma_to_nt(recur_current_ma, self.profile.coil_constant_nt_per_ma)
    }

    pub fn readback(&self, measured_total_ma: f64) -> Result<(f64, f64), MagError> {
        let recur_ma = self.readback_recur_current_ma(measured_total_ma)?;
        let recur_nt = self.readback_recur_field_nt(recur_ma);
        Ok((recur_ma, recur_nt))
    }

    pub fn current_command_preview(&self) -> Option<String> {
        self.total_current_ma
            .and_then(|ma| format_current_command_from_ma(ma).ok())
    }
}

// ---------------------------------------------------------------------------
// Workflow plan builders (Mag-M1)
// ---------------------------------------------------------------------------

/// Build an output-on-zero-mode command plan.
pub fn build_output_on_zero_mode_plan(axis: &MaynuoAxisProfile) -> MaynuoCommandPlan {
    MaynuoCommandPlan {
        schema_version: "0.2.0".into(),
        kind: "maynuo_command_plan".into(),
        id: format!("maynuo_output_on_zero_{}", axis.axis_id),
        name: "Maynuo M8812 Output On (Zero Mode) Plan".into(),
        description: Some("Turn output on at zero current for zero measurement".into()),
        axis_id: axis.axis_id.clone(),
        profile_id: "maynuo_m8812_lab_xyz".into(),
        executable: false,
        executable_reason: "Mag-M1 mock-only: requires Mag-M2B backend".into(),
        shutdown_mode: None,
        expected_duration_ms: Some(100),
        commands: vec![cmd_entry(
            1,
            "set_output",
            "OUTP 1",
            false,
            "none",
            "mag_output_enabled",
            false,
            Some(100),
            None,
        )],
    }
}

/// Build a measure-zero-current command plan.
pub fn build_measure_zero_current_plan(axis: &MaynuoAxisProfile) -> MaynuoCommandPlan {
    MaynuoCommandPlan {
        schema_version: "0.2.0".into(),
        kind: "maynuo_command_plan".into(),
        id: format!("maynuo_measure_zero_{}", axis.axis_id),
        name: "Maynuo M8812 Measure Zero Current Plan".into(),
        description: Some("Query current to capture zero baseline".into()),
        axis_id: axis.axis_id.clone(),
        profile_id: "maynuo_m8812_lab_xyz".into(),
        executable: false,
        executable_reason: "Mag-M1 mock-only: requires Mag-M2B backend".into(),
        shutdown_mode: None,
        expected_duration_ms: Some(300),
        commands: vec![cmd_entry(
            1,
            "query_current",
            "MEAS:CURR?",
            true,
            "float_ampere",
            "mag_current_queried",
            true,
            None,
            Some(300),
        )],
    }
}

/// Build a lock-zero event record (pure event, no SCPI command).
pub fn build_lock_zero_event(axis: &MaynuoAxisProfile, zero_current_ma: f64) -> MaynuoCommandPlan {
    MaynuoCommandPlan {
        schema_version: "0.2.0".into(),
        kind: "maynuo_command_plan".into(),
        id: format!("maynuo_lock_zero_{}", axis.axis_id),
        name: "Maynuo M8812 Lock Zero Event".into(),
        description: Some(format!(
            "Lock-zero enabled with zero_current_ma={zero_current_ma:.3}"
        )),
        axis_id: axis.axis_id.clone(),
        profile_id: "maynuo_m8812_lab_xyz".into(),
        executable: false,
        executable_reason: "Mag-M1 mock-only: lock-zero is a state transition".into(),
        shutdown_mode: None,
        expected_duration_ms: Some(0),
        commands: vec![],
    }
}

/// Build a recurrent-field setpoint command plan.
pub fn build_recur_field_setpoint_plan(
    axis: &MaynuoAxisProfile,
    target_field_nt: f64,
    zero_current_ma: f64,
) -> Result<MaynuoCommandPlan, MagError> {
    let recur_ma = nt_to_ma(target_field_nt, axis.coil_constant_nt_per_ma);
    let total_ma = zero_current_ma + recur_ma;
    if !target_field_nt.is_finite() {
        return Err(MagError::NonFiniteValue {
            field: "target_field_nt".into(),
            value: target_field_nt,
        });
    }
    if total_ma < 0.0 {
        return Err(MagError::SafetyViolation {
            message: format!("negative total current {total_ma:.5} mA"),
        });
    }
    if total_ma > axis.max_current_ma {
        return Err(MagError::TotalCurrentOverLimit {
            axis_id: axis.axis_id.clone(),
            total_ma,
            limit_ma: axis.max_current_ma,
        });
    }
    let total_a = total_ma / 1000.0;
    Ok(MaynuoCommandPlan {
        schema_version: "0.2.0".into(),
        kind: "maynuo_command_plan".into(),
        id: format!("maynuo_recur_field_{}", axis.axis_id),
        name: "Maynuo M8812 Recurrent Field Setpoint Plan".into(),
        description: Some(format!("Target field={target_field_nt:.2} nT, recur={recur_ma:.5} mA, zero={zero_current_ma:.3} mA, total={total_ma:.5} mA")),
        axis_id: axis.axis_id.clone(),
        profile_id: "maynuo_m8812_lab_xyz".into(),
        executable: false,
        executable_reason: "Mag-M1 mock-only: requires Mag-M3 backend".into(),
        shutdown_mode: None,
        expected_duration_ms: Some(50),
        commands: vec![cmd_entry(1, "set_current", &format!("CURR {total_a:.5}"), false, "none", "mag_recur_current_set", false, Some(50), None)],
    })
}

/// Build a recurrent-current setpoint command plan.
pub fn build_recur_current_setpoint_plan(
    axis: &MaynuoAxisProfile,
    recur_current_ma: f64,
    zero_current_ma: f64,
) -> Result<MaynuoCommandPlan, MagError> {
    let total_ma = zero_current_ma + recur_current_ma;
    if !recur_current_ma.is_finite() {
        return Err(MagError::NonFiniteValue {
            field: "recur_current_ma".into(),
            value: recur_current_ma,
        });
    }
    if total_ma < 0.0 {
        return Err(MagError::SafetyViolation {
            message: format!("negative total current {total_ma:.5} mA"),
        });
    }
    if total_ma > axis.max_current_ma {
        return Err(MagError::TotalCurrentOverLimit {
            axis_id: axis.axis_id.clone(),
            total_ma,
            limit_ma: axis.max_current_ma,
        });
    }
    let total_a = total_ma / 1000.0;
    Ok(MaynuoCommandPlan {
        schema_version: "0.2.0".into(),
        kind: "maynuo_command_plan".into(),
        id: format!("maynuo_recur_current_{}", axis.axis_id),
        name: "Maynuo M8812 Recurrent Current Setpoint Plan".into(),
        description: Some(format!("Recur current={recur_current_ma:.5} mA, zero={zero_current_ma:.3} mA, total={total_ma:.5} mA")),
        axis_id: axis.axis_id.clone(),
        profile_id: "maynuo_m8812_lab_xyz".into(),
        executable: false,
        executable_reason: "Mag-M1 mock-only: requires Mag-M3 backend".into(),
        shutdown_mode: None,
        expected_duration_ms: Some(50),
        commands: vec![cmd_entry(1, "set_current", &format!("CURR {total_a:.5}"), false, "none", "mag_recur_current_set", false, Some(50), None)],
    })
}

/// Build a readback query plan with expected reconstruction.
pub fn build_readback_recur_state_plan(
    axis: &MaynuoAxisProfile,
    _measured_total_current_ma: f64,
    _zero_current_ma: f64,
    _lock_zero: bool,
) -> Result<MaynuoCommandPlan, MagError> {
    Ok(MaynuoCommandPlan {
        schema_version: "0.2.0".into(),
        kind: "maynuo_command_plan".into(),
        id: format!("maynuo_readback_recur_{}", axis.axis_id),
        name: "Maynuo M8812 Readback Recur State Plan".into(),
        description: Some("Query current and reconstruct recur state from measurement".into()),
        axis_id: axis.axis_id.clone(),
        profile_id: "maynuo_m8812_lab_xyz".into(),
        executable: false,
        executable_reason: "Mag-M1 mock-only: requires Mag-M3 backend".into(),
        shutdown_mode: None,
        expected_duration_ms: Some(300),
        commands: vec![cmd_entry(
            1,
            "query_current",
            "MEAS:CURR?",
            true,
            "float_ampere",
            "mag_current_queried_for_readback",
            true,
            None,
            Some(300),
        )],
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
            display_name: Some("X Axis".into()),
            last_known_port_name: "COM4".into(),
            device_model: "MAYNUO M8812".into(),
            sn_tail: "2020".into(),
            expected_idn: "MAYNUO,M8812,080020960220402020,V2.7".into(),
            coil_constant_nt_per_ma: 143.26,
            gain_t_per_a: 1.4326e-4,
            zero_offset_ma: 0.0,
            zero_offset_a: 0.0,
            output_default: false,
            max_current_ma: 2000.0,
            max_current_a: 2.0,
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
            description: Some("Verified device fingerprint mapping for lab Maynuo M8812 XYZ magnetic axes.".into()),
            serial_settings: MaynuoSerialSettings::default(),
            axes: MaynuoAxes {
                x: example_maynuo_axis_profile(),
                y: MaynuoAxisProfile {
                    axis_id: "mag_y".into(),
                    display_name: Some("Y Axis".into()),
                    last_known_port_name: "COM6".into(),
                    device_model: "MAYNUO M8812".into(),
                    sn_tail: "2022".into(),
                    expected_idn: "MAYNUO,M8812,080020960220402022,V2.7".into(),
                    coil_constant_nt_per_ma: 141.77,
                    gain_t_per_a: 1.4177e-4,
                    zero_offset_ma: 0.0,
                    zero_offset_a: 0.0,
                    output_default: false,
                    max_current_ma: 2000.0,
                    max_current_a: 2.0,
                    voltage_v: 75,
                },
                z: MaynuoAxisProfile {
                    axis_id: "mag_z".into(),
                    display_name: Some("Z Axis".into()),
                    last_known_port_name: "COM3".into(),
                    device_model: "MAYNUO M8812".into(),
                    sn_tail: "2003".into(),
                    expected_idn: "MAYNUO,M8812,080020960220402003,V2.7".into(),
                    coil_constant_nt_per_ma: 156.15,
                    gain_t_per_a: 1.5615e-4,
                    zero_offset_ma: 0.0,
                    zero_offset_a: 0.0,
                    output_default: false,
                    max_current_ma: 2000.0,
                    max_current_a: 2.0,
                    voltage_v: 75,
                },
            },
            safety_policy_id: "mag_safety_lab_default".into(),
            calibration_date: "2026-05-15".into(),
            verified: true,
            verified_by: Some("reverse_analysis_agent".into()),
            source: Some("reverse_application/reverse_output/para.xml".into()),
            verification: Some(VerificationMetadata {
                method: "power_cycle_identification".into(),
                date: "2026-06-01".into(),
                verified_by: "operator_power_cycle_test".into(),
                result: "X=080020960220402020, Y=080020960220402022, Z=080020960220402003".into(),
                note: Some("Port paths are dynamic per session. Only SN should be used for device binding.".into()),
            }),
            note: Some("POWER_MAX_CURR = 2000 mA hardware limit (M8812 0-2A spec). Only permitted micro-test current is 10 mA.".into()),
        }
    }

    // -----------------------------------------------------------------------
    // Maynuo axes profile to coil matrix
    // -----------------------------------------------------------------------

    #[test]
    fn maynuo_profile_to_coil_matrix_is_diagonal() {
        let profile = example_maynuo_axes_profile();
        let cm = profile.try_to_coil_matrix().unwrap();

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

        // Condition number = max(|kx|,|ky|,|kz|) / min(|kx|,|ky|,|kz|)
        // = 1.5615e-4 / 1.4177e-4 ≈ 1.1015
        let expected_cond = 1.5615e-4 / 1.4177e-4;
        assert!((cm.condition_number() - expected_cond).abs() < 0.01);
    }

    #[test]
    fn maynuo_coil_matrix_roundtrip() {
        let profile = example_maynuo_axes_profile();
        let cm = profile.try_to_coil_matrix().unwrap();

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

        assert_eq!(plan.commands.len(), 9);
        assert_eq!(plan.commands[0].seq, 1);
        assert_eq!(plan.commands[0].scpi, "*IDN?");
        assert!(plan.commands[0].expects_response);
        assert_eq!(plan.commands[1].scpi, "SYST:REM");
        assert!(!plan.commands[1].expects_response);
        assert_eq!(plan.commands[2].scpi, "VOLT 75");
        assert_eq!(plan.commands[3].scpi, "VOLT:PROT 75");
        assert_eq!(plan.commands[4].scpi, "CURR 0.00000");
        assert_eq!(plan.commands[5].scpi, "OUTP 0");
        assert_eq!(plan.commands[6].scpi, "MEAS:CURR?");
        assert!(plan.commands[6].expects_response);
        assert_eq!(plan.commands[7].scpi, "OUTP 0");
        assert_eq!(plan.commands[8].scpi, "SYST:LOC");
        // Set commands should not claim ACK
        for entry in &plan.commands {
            if !entry.expects_response {
                assert_eq!(entry.expected_response_shape, "none");
            }
        }
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
        assert!(plan.commands[0].expects_response);
        assert_eq!(plan.commands[1].scpi, "SYST:REM");
        assert!(!plan.commands[1].expects_response);
        assert_eq!(plan.commands[2].scpi, "VOLT 75");
        assert_eq!(plan.commands[3].scpi, "CURR 0.01000");
        assert_eq!(plan.commands[4].scpi, "OUTP 1");
        assert_eq!(plan.commands[5].scpi, "MEAS:CURR?");
        assert!(plan.commands[5].expects_response);
        assert_eq!(plan.commands[6].scpi, "CURR 0.00000");
        assert_eq!(plan.commands[7].scpi, "OUTP 0");
        assert_eq!(plan.commands[8].scpi, "SYST:LOC");
        // Shutdown mode is verified_normal
        assert_eq!(plan.shutdown_mode.as_deref(), Some("verified_normal"));
        // Set commands should not claim ACK
        for entry in &plan.commands {
            if !entry.expects_response {
                assert_eq!(entry.expected_response_shape, "none");
            }
        }
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
            profile.axes.x.coil_constant_nt_per_ma,
            back.axes.x.coil_constant_nt_per_ma
        );
        assert_eq!(
            profile.axes.y.coil_constant_nt_per_ma,
            back.axes.y.coil_constant_nt_per_ma
        );
        assert_eq!(
            profile.axes.z.coil_constant_nt_per_ma,
            back.axes.z.coil_constant_nt_per_ma
        );
        assert_eq!(
            profile.serial_settings.baudrate,
            back.serial_settings.baudrate
        );
        assert_eq!(profile.serial_settings.dtr, back.serial_settings.dtr);
        // Verification metadata survives round-trip
        assert_eq!(profile.verification, back.verification);
        assert_eq!(profile.source, back.source);
        assert_eq!(profile.note, back.note);
        // port_name alias deserialization
        assert_eq!(back.axes.x.last_known_port_name, "COM4");
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
        assert_eq!(
            plan.commands[0].expects_response,
            back.commands[0].expects_response
        );
        assert_eq!(plan.shutdown_mode, back.shutdown_mode);
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
        let profile = example_maynuo_axes_profile();
        let _ = build_safe_init_plan(&axis);
        let _ = build_10ma_microtest_plan(&axis).unwrap();
        let _ = build_query_current_plan(&axis);
        let _ = build_verified_normal_shutdown_plan(&axis);
        let _ = build_emergency_shutdown_plan(&axis);
        let _ = profile.try_to_coil_matrix().unwrap();
        // Reaching here proves no serial/USB/TCP was invoked.
    }

    // -----------------------------------------------------------------------
    // Shutdown plan tests
    // -----------------------------------------------------------------------

    #[test]
    fn verified_normal_shutdown_order() {
        let axis = example_maynuo_axis_profile();
        let plan = build_verified_normal_shutdown_plan(&axis);
        assert_eq!(plan.shutdown_mode.as_deref(), Some("verified_normal"));
        assert_eq!(plan.commands.len(), 3);
        // CURR 0 → OUTP 0 → SYST:LOC
        assert_eq!(plan.commands[0].scpi, "CURR 0.00000");
        assert_eq!(plan.commands[1].scpi, "OUTP 0");
        assert_eq!(plan.commands[2].scpi, "SYST:LOC");
        // No set commands claim response
        for entry in &plan.commands {
            assert!(!entry.expects_response);
            assert_eq!(entry.expected_response_shape, "none");
        }
    }

    #[test]
    fn emergency_shutdown_order() {
        let axis = example_maynuo_axis_profile();
        let plan = build_emergency_shutdown_plan(&axis);
        assert_eq!(plan.shutdown_mode.as_deref(), Some("emergency"));
        assert_eq!(plan.commands.len(), 3);
        // OUTP 0 → CURR 0 → SYST:LOC (output first!)
        assert_eq!(plan.commands[0].scpi, "OUTP 0");
        assert_eq!(plan.commands[1].scpi, "CURR 0.00000");
        assert_eq!(plan.commands[2].scpi, "SYST:LOC");
        for entry in &plan.commands {
            assert!(!entry.expects_response);
        }
        // Emergency shutdown has zero delay on OUTP 0
        assert_eq!(plan.commands[0].delay_ms, Some(0));
    }

    #[test]
    fn emergency_shutdown_differs_from_normal() {
        let axis = example_maynuo_axis_profile();
        let normal = build_verified_normal_shutdown_plan(&axis);
        let emergency = build_emergency_shutdown_plan(&axis);
        // First commands differ
        assert_ne!(normal.commands[0].scpi, emergency.commands[0].scpi);
        assert_eq!(normal.shutdown_mode.as_deref(), Some("verified_normal"));
        assert_eq!(emergency.shutdown_mode.as_deref(), Some("emergency"));
    }

    // -----------------------------------------------------------------------
    // Command response semantics tests
    // -----------------------------------------------------------------------

    #[test]
    fn set_commands_do_not_expect_response() {
        assert!(!MaynuoCommand::SetRemote.expects_response());
        assert!(!MaynuoCommand::SetLocal.expects_response());
        assert!(!MaynuoCommand::SetVoltage { voltage_v: 75 }.expects_response());
        assert!(!MaynuoCommand::SetCurrent {
            current_a: 0.01,
            current_ma: 10.0
        }
        .expects_response());
        assert!(!MaynuoCommand::SetOutput { on: true }.expects_response());
        assert!(!MaynuoCommand::SetOutput { on: false }.expects_response());
    }

    #[test]
    fn query_commands_expect_response() {
        assert!(MaynuoCommand::Identify.expects_response());
        assert!(MaynuoCommand::QueryCurrent.expects_response());
    }

    #[test]
    fn set_commands_expected_response_shape_is_none() {
        assert_eq!(MaynuoCommand::SetRemote.expected_response_shape(), "none");
        assert_eq!(MaynuoCommand::SetLocal.expected_response_shape(), "none");
        assert_eq!(
            MaynuoCommand::SetVoltage { voltage_v: 75 }.expected_response_shape(),
            "none"
        );
        assert_eq!(
            MaynuoCommand::SetCurrent {
                current_a: 0.01,
                current_ma: 10.0
            }
            .expected_response_shape(),
            "none"
        );
        assert_eq!(
            MaynuoCommand::SetOutput { on: true }.expected_response_shape(),
            "none"
        );
    }

    #[test]
    fn query_commands_expected_response_shape_is_not_none() {
        assert_eq!(
            MaynuoCommand::Identify.expected_response_shape(),
            "MAYNUO,M8812,<SN>,V2.7"
        );
        assert_eq!(
            MaynuoCommand::QueryCurrent.expected_response_shape(),
            "float_ampere"
        );
    }

    // -----------------------------------------------------------------------
    // to_coil_matrix validation tests
    // -----------------------------------------------------------------------

    #[test]
    fn try_to_coil_matrix_rejects_nan_gain() {
        let mut profile = example_maynuo_axes_profile();
        profile.axes.x.gain_t_per_a = f64::NAN;
        let result = profile.try_to_coil_matrix();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::NonFiniteValue { .. }
        ));
    }

    #[test]
    fn try_to_coil_matrix_rejects_inf_gain() {
        let mut profile = example_maynuo_axes_profile();
        profile.axes.y.gain_t_per_a = f64::INFINITY;
        let result = profile.try_to_coil_matrix();
        assert!(result.is_err());
    }

    #[test]
    fn try_to_coil_matrix_rejects_zero_gain() {
        let mut profile = example_maynuo_axes_profile();
        profile.axes.z.gain_t_per_a = 0.0;
        let result = profile.try_to_coil_matrix();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MagError::CalibrationMissing { .. }
        ));
    }

    #[test]
    fn try_to_coil_matrix_rejects_negative_gain() {
        let mut profile = example_maynuo_axes_profile();
        profile.axes.x.gain_t_per_a = -1.0;
        let result = profile.try_to_coil_matrix();
        assert!(result.is_err());
    }

    #[test]
    fn try_to_coil_matrix_rejects_non_finite_offset() {
        let mut profile = example_maynuo_axes_profile();
        profile.axes.y.zero_offset_a = f64::NAN;
        let result = profile.try_to_coil_matrix();
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Port binding semantics tests
    // -----------------------------------------------------------------------

    #[test]
    fn port_name_is_not_stable_binding_key() {
        let profile = example_maynuo_axes_profile();
        // Port paths are documented as hints only
        let json = serde_json::to_string(&profile).unwrap();
        // The JSON uses last_known_port_name, not port_name (the old name)
        assert!(
            json.contains("last_known_port_name"),
            "JSON should use last_known_port_name"
        );
        // SN mapping is present as the stable identity
        assert!(
            json.contains("080020960220402020"),
            "X-axis SN must be present"
        );
        assert!(
            json.contains("080020960220402022"),
            "Y-axis SN must be present"
        );
        assert!(
            json.contains("080020960220402003"),
            "Z-axis SN must be present"
        );
        // expected_idn is the stable binding key
        for axis in [&profile.axes.x, &profile.axes.y, &profile.axes.z] {
            assert!(!axis.last_known_port_name.is_empty());
            assert!(axis.last_known_port_name.starts_with("COM"));
            // SN tail is embedded in expected_idn
            assert!(axis.expected_idn.contains(&axis.sn_tail));
        }
    }

    // -----------------------------------------------------------------------
    // Example file deserialization tests
    // -----------------------------------------------------------------------

    #[test]
    fn deserialize_maynuo_axes_example_json() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/magnetic/maynuo_m8812_axes.example.json"
        );
        let content = std::fs::read_to_string(path).unwrap();
        let profile: MaynuoAxesProfile = serde_json::from_str(&content).unwrap();

        assert_eq!(profile.kind, "maynuo_axes_profile");
        assert_eq!(profile.id, "maynuo_m8812_lab_xyz");

        // Axes are nested
        assert_eq!(profile.axes.x.axis_id, "mag_x");
        assert_eq!(profile.axes.y.axis_id, "mag_y");
        assert_eq!(profile.axes.z.axis_id, "mag_z");

        // port_name alias works
        assert_eq!(profile.axes.x.last_known_port_name, "COM4");

        // Verification metadata survives
        let v = profile.verification.as_ref().unwrap();
        assert_eq!(v.method, "power_cycle_identification");
        assert_eq!(v.date, "2026-06-01");
        assert_eq!(v.verified_by, "operator_power_cycle_test");
        assert!(v.result.contains("080020960220402020"));

        // Re-serialize preserves verification
        let roundtrip_json = serde_json::to_string_pretty(&profile).unwrap();
        let back: MaynuoAxesProfile = serde_json::from_str(&roundtrip_json).unwrap();
        assert_eq!(
            back.verification.as_ref().unwrap().method,
            "power_cycle_identification"
        );
        assert_eq!(back.verification.as_ref().unwrap().result, v.result);
    }

    #[test]
    fn deserialize_safe_init_plan_example_json() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/magnetic/maynuo_m8812_safe_init_plan.example.json"
        );
        let content = std::fs::read_to_string(path).unwrap();
        let plan: MaynuoCommandPlan = serde_json::from_str(&content).unwrap();

        assert_eq!(plan.kind, "maynuo_command_plan");
        assert_eq!(plan.id, "maynuo_safe_init_plan");
        assert!(!plan.executable);
        assert_eq!(plan.commands.len(), 9);

        // Query commands expect response; set commands do not
        let idn = &plan.commands[0];
        assert_eq!(idn.scpi, "*IDN?");
        assert!(idn.expects_response);
        assert_eq!(idn.expected_response_shape, "MAYNUO,M8812,<SN>,V2.7");

        let remote = &plan.commands[1];
        assert_eq!(remote.scpi, "SYST:REM");
        assert!(!remote.expects_response);
        assert_eq!(remote.expected_response_shape, "none");

        let volt_prot = &plan.commands[3];
        assert_eq!(volt_prot.scpi, "VOLT:PROT 75");
        assert!(!volt_prot.expects_response);

        let query = &plan.commands[6];
        assert_eq!(query.scpi, "MEAS:CURR?");
        assert!(query.expects_response);
        assert_eq!(query.expected_response_shape, "float_ampere");
    }

    #[test]
    fn deserialize_10ma_microtest_plan_example_json() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/magnetic/maynuo_m8812_10ma_microtest_plan.example.json"
        );
        let content = std::fs::read_to_string(path).unwrap();
        let plan: MaynuoCommandPlan = serde_json::from_str(&content).unwrap();

        assert_eq!(plan.kind, "maynuo_command_plan");
        assert!(!plan.executable);
        assert_eq!(plan.commands.len(), 9);
        assert_eq!(plan.commands[3].scpi, "CURR 0.01000");

        // Shutdown mode must be set
        assert_eq!(plan.shutdown_mode.as_deref(), Some("verified_normal"));

        // Shutdown sequence: CURR 0 → OUTP 0 → SYST:LOC (verified normal)
        let shutdown = &plan.commands[6..];
        assert_eq!(shutdown[0].scpi, "CURR 0.00000");
        assert_eq!(shutdown[1].scpi, "OUTP 0");
        assert_eq!(shutdown[2].scpi, "SYST:LOC");
    }

    #[test]
    fn deserialize_gui_contract_example_json() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/magnetic/maynuo_m8812_gui_contract.example.json"
        );
        let content = std::fs::read_to_string(path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(v["kind"], "maynuo_gui_contract");
        assert_eq!(v["gui_milestone"], "M0.5");

        // All axis cards must have disabled real controls
        for card in v["axis_cards"].as_array().unwrap() {
            assert_eq!(card["output"]["settable"], false);
            assert_eq!(card["target_current"]["settable"], false);
            assert_ne!(card["target_current"]["disabled_reason"], "");
        }

        // Global emergency stop is not available
        assert_eq!(v["global_state"]["emergency_stop_available"], false);
        assert_ne!(v["global_state"]["emergency_stop_disabled_reason"], "");

        // No executable flag set
        let forbidden = v["backend_api_contract"]["forbidden_patterns"]
            .as_array()
            .unwrap();
        assert!(!forbidden.is_empty());

        // Backend commands available must be a subset with none enabling hardware
        let cmds = v["backend_api_contract"]["tauri_commands"]
            .as_array()
            .unwrap();
        let real_control_cmds: Vec<_> = cmds
            .iter()
            .filter(|c| c["available"].as_bool() == Some(true))
            .collect();
        // In M0.5, only magnetic_get_mock_state is available
        for cmd in &real_control_cmds {
            let name = cmd["name"].as_str().unwrap();
            assert!(
                name.contains("mock") || name.contains("preview"),
                "Command {name} should not be available in M0.5"
            );
        }
    }

    #[test]
    fn gui_contract_no_executable_flag() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/magnetic/maynuo_m8812_gui_contract.example.json"
        );
        let content = std::fs::read_to_string(path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();

        // No executable flag anywhere in the contract
        assert!(!content.contains("\"executable\": true"));

        // Forbidden patterns are documented but are NOT actual code paths.
        // Verify the forbidden_patterns section exists (it documents what
        // React MUST NOT do — it is not itself executable).
        let forbidden = v["backend_api_contract"]["forbidden_patterns"]
            .as_array()
            .unwrap();
        assert!(forbidden.len() >= 6);

        // The contract describes display-only payloads; it contains no
        // executable handlers or I/O instructions.
    }

    // -----------------------------------------------------------------------
    // Mag-M1: SN-based discovery matching
    // -----------------------------------------------------------------------

    #[test]
    fn parse_maynuo_idn_standard() {
        let p = parse_maynuo_idn("MAYNUO,M8812,080020960220402020,V2.7").unwrap();
        assert_eq!(p.manufacturer, "MAYNUO");
        assert_eq!(p.model, "M8812");
        assert_eq!(p.serial_number, "080020960220402020");
        assert_eq!(p.firmware.as_deref(), Some("V2.7"));
    }

    #[test]
    fn parse_maynuo_idn_rejects_empty() {
        assert!(matches!(
            parse_maynuo_idn(""),
            Err(MagError::MalformedIdn { .. })
        ));
    }

    #[test]
    fn parse_maynuo_idn_rejects_fewer_than_3_fields() {
        assert!(matches!(
            parse_maynuo_idn("MAYNUO,M8812"),
            Err(MagError::MalformedIdn { .. })
        ));
    }

    #[test]
    fn parse_maynuo_idn_rejects_wrong_manufacturer() {
        assert!(matches!(
            parse_maynuo_idn("OTHER,M8812,SN123,V2.7"),
            Err(MagError::MalformedIdn { .. })
        ));
    }

    #[test]
    fn parse_maynuo_idn_rejects_wrong_model() {
        assert!(matches!(
            parse_maynuo_idn("MAYNUO,M9999,SN123,V2.7"),
            Err(MagError::MalformedIdn { .. })
        ));
    }

    #[test]
    fn parse_maynuo_idn_rejects_empty_sn() {
        assert!(matches!(
            parse_maynuo_idn("MAYNUO,M8812,,V2.7"),
            Err(MagError::MalformedIdn { .. })
        ));
    }

    #[test]
    fn expected_sn_from_idn_extracts_sn() {
        let sn = expected_sn_from_idn("MAYNUO,M8812,080020960220402020,V2.7").unwrap();
        assert_eq!(sn, "080020960220402020");
    }

    #[test]
    fn match_all_three_axes_by_idn() {
        let profile = example_maynuo_axes_profile();
        let observed = vec![
            "MAYNUO,M8812,080020960220402020,V2.7".to_string(),
            "MAYNUO,M8812,080020960220402022,V2.7".to_string(),
            "MAYNUO,M8812,080020960220402003,V2.7".to_string(),
        ];
        let matched = match_axes_by_idn(&profile, &observed).unwrap();
        assert_eq!(
            matched.get("mag_x").unwrap(),
            "MAYNUO,M8812,080020960220402020,V2.7"
        );
        assert_eq!(
            matched.get("mag_y").unwrap(),
            "MAYNUO,M8812,080020960220402022,V2.7"
        );
        assert_eq!(
            matched.get("mag_z").unwrap(),
            "MAYNUO,M8812,080020960220402003,V2.7"
        );
    }

    #[test]
    fn unknown_sn_rejected() {
        let profile = example_maynuo_axes_profile();
        let observed = vec!["MAYNUO,M8812,090000000000000099,V2.7".to_string()];
        let result = match_axes_by_idn(&profile, &observed);
        assert!(matches!(
            result.unwrap_err(),
            MagError::UnknownSerialNumber { .. }
        ));
    }

    #[test]
    fn duplicate_sn_rejected() {
        let profile = example_maynuo_axes_profile();
        let observed = vec![
            "MAYNUO,M8812,080020960220402020,V2.7".to_string(),
            "MAYNUO,M8812,080020960220402020,V2.7".to_string(),
        ];
        let result = match_axes_by_idn(&profile, &observed);
        assert!(result.is_err());
    }

    #[test]
    fn missing_axis_rejected() {
        let profile = example_maynuo_axes_profile();
        let observed = vec!["MAYNUO,M8812,080020960220402020,V2.7".to_string()];
        let result = match_axes_by_idn(&profile, &observed);
        assert!(matches!(
            result.unwrap_err(),
            MagError::AxisNotDiscovered { .. }
        ));
    }

    // -----------------------------------------------------------------------
    // Mag-M1: Axis state machine transitions
    // -----------------------------------------------------------------------

    fn example_runner() -> MaynuoAxisRunner {
        MaynuoAxisRunner::new(example_maynuo_axis_profile())
    }

    #[test]
    fn state_machine_unknown_to_discovered() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        assert!(matches!(r.state, MaynuoAxisState::Discovered { .. }));
    }

    #[test]
    fn state_machine_wrong_sn_rejected() {
        let mut r = example_runner();
        let result = r.apply_discovered("MAYNUO,M8812,090000000000000099,V2.7");
        assert!(matches!(
            result.unwrap_err(),
            MagError::UnknownSerialNumber { .. }
        ));
    }

    #[test]
    fn state_machine_discovered_to_mapped() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        assert!(matches!(r.state, MaynuoAxisState::AxisMapped { .. }));
    }

    #[test]
    fn state_machine_mapped_to_init() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        assert!(matches!(
            r.state,
            MaynuoAxisState::InitializedOutputOff { .. }
        ));
    }

    #[test]
    fn state_machine_init_to_output_on_zero() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        assert!(matches!(r.state, MaynuoAxisState::OutputOnZeroMode { .. }));
        assert!(r.output);
    }

    #[test]
    fn output_before_init_rejected() {
        let mut r = example_runner();
        // Try to turn output on without safe init
        let result = r.apply_output_on_zero_mode();
        assert!(matches!(
            result.unwrap_err(),
            MagError::OutputBeforeInit { .. }
        ));
    }

    #[test]
    fn state_machine_zero_measured() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(0.15).unwrap();
        assert!(
            matches!(r.state, MaynuoAxisState::ZeroMeasured { zero_current_ma, .. } if (zero_current_ma - 0.15).abs() < 0.001)
        );
        assert!((r.zero_current_ma.unwrap() - 0.15).abs() < 0.001);
    }

    #[test]
    fn zero_measured_nan_rejected() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        let result = r.apply_zero_measured(f64::NAN);
        assert!(result.is_err());
    }

    #[test]
    fn lock_zero_before_measurement_rejected() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        // Skip zero measurement, try lock-zero
        let result = r.apply_lock_zero();
        assert!(matches!(
            result.unwrap_err(),
            MagError::LockZeroBeforeMeasurement { .. }
        ));
    }

    #[test]
    fn state_machine_zero_locked() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(0.12).unwrap();
        r.apply_lock_zero().unwrap();
        assert!(matches!(r.state, MaynuoAxisState::ZeroLocked { .. }));
        assert!(r.lock_zero);
        assert!((r.zero_current_ma.unwrap() - 0.12).abs() < 0.001);
    }

    #[test]
    fn recur_before_lock_zero_rejected() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(0.10).unwrap();
        // No lock_zero — try recur field
        let result = r.apply_recur_setpoint_planned_from_field(1000.0);
        assert!(matches!(
            result.unwrap_err(),
            MagError::RecurBeforeLockZero { .. }
        ));
    }

    #[test]
    fn field_1000_nt_on_x() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(0.0).unwrap();
        r.apply_lock_zero().unwrap();
        r.apply_recur_setpoint_planned_from_field(1000.0).unwrap();

        // 1000 nT / 143.26 nT/mA ≈ 6.9803 mA
        let expected_recur_ma = 1000.0 / 143.26;
        assert!((r.recur_current_ma.unwrap() - expected_recur_ma).abs() < 0.01);
        assert!((r.recur_field_nt.unwrap() - 1000.0).abs() < 0.01);
        // Total = zero + recur = 0 + 6.98 ≈ 6.98 mA
        assert!((r.total_current_ma.unwrap() - expected_recur_ma).abs() < 0.01);
    }

    #[test]
    fn field_1000_nt_on_y() {
        let mut profile = example_maynuo_axis_profile();
        profile.axis_id = "mag_y".into();
        profile.coil_constant_nt_per_ma = 141.77;
        profile.expected_idn = "MAYNUO,M8812,080020960220402022,V2.7".into();
        profile.sn_tail = "2022".into();

        let mut r = MaynuoAxisRunner::new(profile);
        r.apply_discovered("MAYNUO,M8812,080020960220402022,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(0.0).unwrap();
        r.apply_lock_zero().unwrap();
        r.apply_recur_setpoint_planned_from_field(1000.0).unwrap();

        let expected_recur_ma = 1000.0 / 141.77;
        assert!((r.recur_current_ma.unwrap() - expected_recur_ma).abs() < 0.01);
    }

    #[test]
    fn field_1000_nt_on_z() {
        let mut profile = example_maynuo_axis_profile();
        profile.axis_id = "mag_z".into();
        profile.coil_constant_nt_per_ma = 156.15;
        profile.expected_idn = "MAYNUO,M8812,080020960220402003,V2.7".into();
        profile.sn_tail = "2003".into();

        let mut r = MaynuoAxisRunner::new(profile);
        r.apply_discovered("MAYNUO,M8812,080020960220402003,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(0.0).unwrap();
        r.apply_lock_zero().unwrap();
        r.apply_recur_setpoint_planned_from_field(1000.0).unwrap();

        let expected_recur_ma = 1000.0 / 156.15;
        assert!((r.recur_current_ma.unwrap() - expected_recur_ma).abs() < 0.01);
    }

    #[test]
    fn total_current_equals_zero_plus_recur() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(2.5).unwrap(); // zero = 2.5 mA
        r.apply_lock_zero().unwrap();
        r.apply_recur_setpoint_planned_from_field(1000.0).unwrap();

        let zero = r.zero_current_ma.unwrap();
        let recur = r.recur_current_ma.unwrap();
        let total = r.total_current_ma.unwrap();
        assert!((total - (zero + recur)).abs() < 0.001);
    }

    #[test]
    fn command_preview_uses_curr_format() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(0.0).unwrap();
        r.apply_lock_zero().unwrap();
        r.apply_recur_setpoint_planned_from_field(1000.0).unwrap();
        r.apply_recur_setpoint_applied_mock().unwrap();

        let preview = r.current_command_preview().unwrap();
        assert!(
            preview.starts_with("CURR "),
            "Expected CURR command, got {preview}"
        );
        // Should be 5 decimal places
        let parts: Vec<&str> = preview.split_whitespace().collect();
        assert_eq!(parts.len(), 2);
        let value_str = parts[1];
        assert!(value_str.contains('.'), "Expected decimal, got {value_str}");
    }

    #[test]
    fn readback_reconstructs_recur_current() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(1.0).unwrap(); // zero = 1 mA
        r.apply_lock_zero().unwrap();
        r.apply_recur_setpoint_planned_from_field(1000.0).unwrap();

        // Simulate readback: measured total = 8 mA, zero = 1 mA, recur = 7 mA
        let (recur_ma, recur_nt) = r.readback(8.0).unwrap();
        assert!((recur_ma - 7.0).abs() < 0.01);
        // Field = recur * coil_constant = 7 * 143.26 ≈ 1002.82 nT
        let expected_nt = 7.0 * 143.26;
        assert!((recur_nt - expected_nt).abs() < 0.1);
    }

    #[test]
    fn readback_without_lock_zero_returns_zero_recur() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(5.0).unwrap();
        // lock_zero is still false

        let (recur_ma, _) = r.readback(10.0).unwrap();
        assert_eq!(recur_ma, 0.0);
    }

    #[test]
    fn total_current_over_limit_rejected() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(4500.0).unwrap(); // zero near limit
        r.apply_lock_zero().unwrap();
        // Target field that would push total > 5000 mA
        let result = r.apply_recur_setpoint_planned_from_field(100_000.0);
        assert!(matches!(
            result.unwrap_err(),
            MagError::TotalCurrentOverLimit { .. }
        ));
    }

    #[test]
    fn nan_field_rejected() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(0.0).unwrap();
        r.apply_lock_zero().unwrap();
        let result = r.apply_recur_setpoint_planned_from_field(f64::NAN);
        assert!(matches!(
            result.unwrap_err(),
            MagError::NonFiniteValue { .. }
        ));
    }

    #[test]
    fn emergency_shutdown_valid_from_any_state() {
        let mut r = example_runner();
        r.apply_shutdown_emergency().unwrap();
        assert!(matches!(r.state, MaynuoAxisState::ShutdownEmergency { .. }));
        assert!(!r.output);
    }

    #[test]
    fn normal_shutdown_from_output_on_zero() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_shutdown_normal().unwrap();
        assert!(matches!(r.state, MaynuoAxisState::ShutdownNormal { .. }));
        assert!(!r.output);
    }

    #[test]
    fn setpoint_applied_mock_rejected_before_planned() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(0.0).unwrap();
        r.apply_lock_zero().unwrap();
        // No recur setpoint planned — try to apply
        let result = r.apply_recur_setpoint_applied_mock();
        assert!(matches!(
            result.unwrap_err(),
            MagError::InvalidStateTransition { .. }
        ));
    }

    #[test]
    fn full_workflow_events_recorded() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(0.15).unwrap();
        r.apply_lock_zero().unwrap();
        r.apply_recur_setpoint_planned_from_field(1000.0).unwrap();
        r.apply_recur_setpoint_applied_mock().unwrap();

        assert!(
            r.events.len() >= 8,
            "Expected at least 8 events, got {}",
            r.events.len()
        );
        // Verify key events are present
        let event_types: Vec<&str> = r.events.iter().map(|e| e.event_type.as_str()).collect();
        assert!(event_types.contains(&"mag_axis_discovered"));
        assert!(event_types.contains(&"mag_axis_mapped"));
        assert!(event_types.contains(&"mag_init_complete"));
        assert!(event_types.contains(&"mag_output_on_zero"));
        assert!(event_types.contains(&"mag_zero_measured"));
        assert!(event_types.contains(&"mag_lock_zero_enabled"));
        assert!(event_types.contains(&"mag_recur_setpoint_planned"));
        assert!(event_types.contains(&"mag_recur_setpoint_applied"));
    }

    #[test]
    fn readback_recur_field_from_current() {
        let r = example_runner();
        // 5 mA * 143.26 nT/mA = 716.3 nT
        let field = r.readback_recur_field_nt(5.0);
        assert!((field - 716.3).abs() < 0.1);
    }

    #[test]
    fn readback_negative_measured_rejected() {
        let r = example_runner();
        let result = r.readback_recur_current_ma(-1.0);
        assert!(result.is_err());
    }

    #[test]
    fn readback_negative_recur_current_preserved() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        r.apply_initialized_output_off().unwrap();
        r.apply_output_on_zero_mode().unwrap();
        r.apply_zero_measured(100.0).unwrap(); // zero = 100 mA
        r.apply_lock_zero().unwrap();
        // measured total = 90 mA → recur = -10 mA (negative means less than zero offset)
        let (recur_ma, recur_nt) = r.readback(90.0).unwrap();
        assert!((recur_ma - (-10.0)).abs() < 0.01);
        assert!(recur_nt < 0.0);
    }

    #[test]
    fn match_axes_rejects_malformed_idn() {
        let profile = example_maynuo_axes_profile();
        let observed = vec!["MAYNUO,M8812".to_string()]; // only 2 fields
        let result = match_axes_by_idn(&profile, &observed);
        assert!(matches!(result.unwrap_err(), MagError::MalformedIdn { .. }));
    }

    // -----------------------------------------------------------------------
    // Mag-M1: Workflow plan builders
    // -----------------------------------------------------------------------

    #[test]
    fn output_on_zero_plan_is_single_outp_1() {
        let axis = example_maynuo_axis_profile();
        let plan = build_output_on_zero_mode_plan(&axis);
        assert_eq!(plan.commands.len(), 1);
        assert_eq!(plan.commands[0].scpi, "OUTP 1");
        assert!(!plan.commands[0].expects_response);
    }

    #[test]
    fn measure_zero_plan_is_single_meas_curr() {
        let axis = example_maynuo_axis_profile();
        let plan = build_measure_zero_current_plan(&axis);
        assert_eq!(plan.commands.len(), 1);
        assert_eq!(plan.commands[0].scpi, "MEAS:CURR?");
        assert!(plan.commands[0].expects_response);
    }

    #[test]
    fn lock_zero_event_has_no_commands() {
        let axis = example_maynuo_axis_profile();
        let plan = build_lock_zero_event(&axis, 0.15);
        assert!(plan.commands.is_empty());
        assert!(plan.description.unwrap().contains("0.150"));
    }

    #[test]
    fn recur_field_setpoint_plan_contains_total() {
        let axis = example_maynuo_axis_profile();
        let plan = build_recur_field_setpoint_plan(&axis, 1000.0, 0.0).unwrap();
        assert_eq!(plan.commands.len(), 1);
        // 1000 / 143.26 ≈ 6.9803 mA, total = 6.9803 mA
        // CURR {6.9803/1000:.5} = CURR 0.00698
        let expected_ma = 1000.0 / 143.26;
        let expected_a = expected_ma / 1000.0;
        assert_eq!(plan.commands[0].scpi, format!("CURR {expected_a:.5}"));
    }

    #[test]
    fn recur_current_setpoint_plan_adds_zero() {
        let axis = example_maynuo_axis_profile();
        let plan = build_recur_current_setpoint_plan(&axis, 20.0, 3.0).unwrap();
        // total = 23 mA = 0.023 A
        assert_eq!(plan.commands[0].scpi, "CURR 0.02300");
    }

    #[test]
    fn recur_field_setpoint_over_limit_rejected() {
        let axis = example_maynuo_axis_profile();
        let result = build_recur_field_setpoint_plan(&axis, 1e9, 4999.0);
        assert!(result.is_err());
    }

    #[test]
    fn readback_plan_has_single_query() {
        let axis = example_maynuo_axis_profile();
        let plan = build_readback_recur_state_plan(&axis, 10.0, 1.0, true).unwrap();
        assert_eq!(plan.commands.len(), 1);
        assert_eq!(plan.commands[0].scpi, "MEAS:CURR?");
        assert!(plan.commands[0].expects_response);
    }

    // -----------------------------------------------------------------------
    // Mag-M1: All plans not executable
    // -----------------------------------------------------------------------

    #[test]
    fn all_mag_m1_plans_are_not_executable() {
        let axis = example_maynuo_axis_profile();
        let plans: Vec<MaynuoCommandPlan> = vec![
            build_output_on_zero_mode_plan(&axis),
            build_measure_zero_current_plan(&axis),
            build_lock_zero_event(&axis, 0.0),
            build_recur_field_setpoint_plan(&axis, 100.0, 0.0).unwrap(),
            build_recur_current_setpoint_plan(&axis, 10.0, 0.0).unwrap(),
            build_readback_recur_state_plan(&axis, 10.0, 0.0, true).unwrap(),
        ];
        for plan in &plans {
            assert!(
                !plan.executable,
                "Plan {} should not be executable",
                plan.id
            );
        }
    }

    // -----------------------------------------------------------------------
    // Mag-M1: MaynuoAxisState serde round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn axis_state_serde_roundtrip() {
        let state = MaynuoAxisState::ZeroLocked {
            axis_id: "mag_x".into(),
            idn: "MAYNUO,M8812,080020960220402020,V2.7".into(),
            zero_current_ma: 0.15,
        };
        let json = serde_json::to_string(&state).unwrap();
        let back: MaynuoAxisState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, back);
    }

    #[test]
    fn axis_runner_serde_roundtrip() {
        let mut r = example_runner();
        r.apply_discovered("MAYNUO,M8812,080020960220402020,V2.7")
            .unwrap();
        r.apply_axis_mapped().unwrap();
        let json = serde_json::to_string(&r).unwrap();
        let back: MaynuoAxisRunner = serde_json::from_str(&json).unwrap();
        assert_eq!(r.axis_id, back.axis_id);
        assert_eq!(r.state, back.state);
    }

    // -----------------------------------------------------------------------
    // Mag-M1: axis_id validation in transitions
    // -----------------------------------------------------------------------

    #[test]
    fn mapped_to_init_without_mapped_is_rejected() {
        let mut r = example_runner();
        // Skip discovered and mapped
        let result = r.apply_initialized_output_off();
        assert!(result.is_err());
    }
}
