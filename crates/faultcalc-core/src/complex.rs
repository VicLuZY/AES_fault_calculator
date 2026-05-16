use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, Neg, Sub, SubAssign};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };
    pub const ONE: Self = Self { re: 1.0, im: 0.0 };

    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn from_polar(r: f64, theta: f64) -> Self {
        Self { re: r * theta.cos(), im: r * theta.sin() }
    }

    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }

    pub fn finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }

    pub fn conj(self) -> Self {
        Self { re: self.re, im: -self.im }
    }

    pub fn inv(self) -> Self {
        let denom = self.re * self.re + self.im * self.im;
        Self { re: self.re / denom, im: -self.im / denom }
    }

    pub fn scale(self, k: f64) -> Self {
        Self { re: self.re * k, im: self.im * k }
    }
}

impl Add for Complex {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self { re: self.re + rhs.re, im: self.im + rhs.im }
    }
}
impl AddAssign for Complex {
    fn add_assign(&mut self, rhs: Self) {
        self.re += rhs.re;
        self.im += rhs.im;
    }
}
impl Sub for Complex {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self { re: self.re - rhs.re, im: self.im - rhs.im }
    }
}
impl SubAssign for Complex {
    fn sub_assign(&mut self, rhs: Self) {
        self.re -= rhs.re;
        self.im -= rhs.im;
    }
}
impl Mul for Complex {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }
}
impl Mul<f64> for Complex {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        self.scale(rhs)
    }
}
impl Div for Complex {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}
impl DivAssign for Complex {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}
impl Neg for Complex {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self { re: -self.re, im: -self.im }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "actual {actual} expected {expected}"
        );
    }

    fn assert_complex_close(actual: Complex, expected: Complex) {
        assert_close(actual.re, expected.re);
        assert_close(actual.im, expected.im);
    }

    #[test]
    fn arithmetic_operations_are_consistent() {
        let a = Complex::new(3.0, 4.0);
        let b = Complex::new(1.5, -2.0);

        assert_complex_close(a + b, Complex::new(4.5, 2.0));
        assert_complex_close(a - b, Complex::new(1.5, 6.0));
        assert_complex_close(-b, Complex::new(-1.5, 2.0));
        assert_complex_close(a.conj(), Complex::new(3.0, -4.0));
        assert_close(a.abs(), 5.0);
    }

    #[test]
    fn multiplication_division_and_inverse_match_hand_calculation() {
        let a = Complex::new(3.0, 4.0);
        let b = Complex::new(1.0, -2.0);

        assert_complex_close(a * b, Complex::new(11.0, -2.0));
        assert_complex_close(b.inv(), Complex::new(0.2, 0.4));
        assert_complex_close(a / b, Complex::new(-1.0, 2.0));
    }

    #[test]
    fn polar_and_scale_helpers_are_deterministic() {
        let z = Complex::from_polar(2.0, std::f64::consts::FRAC_PI_2);
        assert_close(z.re, 0.0);
        assert_close(z.im, 2.0);
        assert_complex_close(z.scale(3.0), Complex::new(0.0, 6.0));
        assert!(z.finite());
    }
}
