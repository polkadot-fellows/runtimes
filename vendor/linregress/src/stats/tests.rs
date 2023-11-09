use super::*;
use crate::assert_almost_eq;

fn beta_reg(a: f64, b: f64, x: f64) -> f64 {
    checked_beta_reg(a, b, x).unwrap()
}

#[test]
fn test_students_t_cdf() {
    assert_eq!(students_t_cdf(0., 1).unwrap(), 0.5);
    assert_eq!(students_t_cdf(0., 2).unwrap(), 0.5);
    assert_almost_eq!(students_t_cdf(1., 1).unwrap(), 0.75, 1e-15);
    assert_almost_eq!(students_t_cdf(-1., 1).unwrap(), 0.25, 1e-15);
    assert_almost_eq!(students_t_cdf(2., 1).unwrap(), 0.852416382349567, 1e-15);
    assert_almost_eq!(students_t_cdf(-2., 1).unwrap(), 0.147583617650433, 1e-15);
    assert_almost_eq!(students_t_cdf(1., 2).unwrap(), 0.788675134594813, 1e-15);
    assert_almost_eq!(students_t_cdf(-1., 2).unwrap(), 0.211324865405187, 1e-15);
    assert_almost_eq!(students_t_cdf(2., 2).unwrap(), 0.908248290463863, 1e-15);
    assert_almost_eq!(students_t_cdf(-2., 2).unwrap(), 0.091751709536137, 1e-15);
}

#[test]
fn test_beta_reg() {
    assert_almost_eq!(beta_reg(0.5, 0.5, 0.5), 0.5, 1e-15);
    assert_eq!(beta_reg(0.5, 0.5, 1.0), 1.0);
    assert_almost_eq!(beta_reg(1.0, 0.5, 0.5), 0.292893218813452475599, 1e-15);
    assert_eq!(beta_reg(1.0, 0.5, 1.0), 1.0);
    assert_almost_eq!(beta_reg(2.5, 0.5, 0.5), 0.07558681842161243795, 1e-16);
    assert_eq!(beta_reg(2.5, 0.5, 1.0), 1.0);
    assert_almost_eq!(beta_reg(0.5, 1.0, 0.5), 0.7071067811865475244, 1e-15);
    assert_eq!(beta_reg(0.5, 1.0, 1.0), 1.0);
    assert_almost_eq!(beta_reg(1.0, 1.0, 0.5), 0.5, 1e-15);
    assert_eq!(beta_reg(1.0, 1.0, 1.0), 1.0);
    assert_almost_eq!(beta_reg(2.5, 1.0, 0.5), 0.1767766952966368811, 1e-15);
    assert_eq!(beta_reg(2.5, 1.0, 1.0), 1.0);
    assert_eq!(beta_reg(0.5, 2.5, 0.5), 0.92441318157838756205);
    assert_eq!(beta_reg(0.5, 2.5, 1.0), 1.0);
    assert_almost_eq!(beta_reg(1.0, 2.5, 0.5), 0.8232233047033631189, 1e-15);
    assert_eq!(beta_reg(1.0, 2.5, 1.0), 1.0);
    assert_almost_eq!(beta_reg(2.5, 2.5, 0.5), 0.5, 1e-15);
    assert_eq!(beta_reg(2.5, 2.5, 1.0), 1.0);
}

#[test]
#[should_panic]
fn test_beta_reg_a_lte_0() {
    beta_reg(0.0, 1.0, 1.0);
}

#[test]
#[should_panic]
fn test_beta_reg_b_lte_0() {
    beta_reg(1.0, 0.0, 1.0);
}

#[test]
#[should_panic]
fn test_beta_reg_x_lt_0() {
    beta_reg(1.0, 1.0, -1.0);
}

#[test]
#[should_panic]
fn test_beta_reg_x_gt_1() {
    beta_reg(1.0, 1.0, 2.0);
}

#[test]
fn test_checked_beta_reg_a_lte_0() {
    assert!(checked_beta_reg(0.0, 1.0, 1.0).is_none());
}

#[test]
fn test_checked_beta_reg_b_lte_0() {
    assert!(checked_beta_reg(1.0, 0.0, 1.0).is_none());
}

#[test]
fn test_checked_beta_reg_x_lt_0() {
    assert!(checked_beta_reg(1.0, 1.0, -1.0).is_none());
}

#[test]
fn test_checked_beta_reg_x_gt_1() {
    assert!(checked_beta_reg(1.0, 1.0, 2.0).is_none());
}
#[test]
fn test_ln_gamma() {
    assert!(ln_gamma(f64::NAN).is_nan());
    assert_eq!(
        ln_gamma(1.000001e-35),
        80.59047725479209894029636783061921392709972287131139201585211
    );
    assert_almost_eq!(
        ln_gamma(1.000001e-10),
        23.02584992988323521564308637407936081168344192865285883337793,
        1e-14
    );
    assert_almost_eq!(
        ln_gamma(1.000001e-5),
        11.51291869289055371493077240324332039045238086972508869965363,
        1e-14
    );
    assert_eq!(
        ln_gamma(1.000001e-2),
        4.599478872433667224554543378460164306444416156144779542513592
    );
    assert_almost_eq!(
        ln_gamma(0.1),
        2.252712651734205959869701646368495118615627222294953765041739,
        1e-14
    );
    assert_almost_eq!(
        ln_gamma(1.0 - 1.0e-14),
        5.772156649015410852768463312546533565566459794933360600e-15,
        1e-15
    );
    assert_almost_eq!(ln_gamma(1.0), 0.0, 1e-15);
    assert_almost_eq!(
        ln_gamma(1.0 + 1.0e-14),
        -5.77215664901524635936177848990288632404978978079827014e-15,
        1e-15
    );
    assert_almost_eq!(
        ln_gamma(1.5),
        -0.12078223763524522234551844578164721225185272790259946836386,
        1e-14
    );
    assert_almost_eq!(
        ln_gamma(f64::consts::PI / 2.0),
        -0.11590380084550241329912089415904874214542604767006895,
        1e-14
    );
    assert_eq!(ln_gamma(2.0), 0.0);
    assert_almost_eq!(
        ln_gamma(2.5),
        0.284682870472919159632494669682701924320137695559894729250145,
        1e-13
    );
    assert_almost_eq!(
        ln_gamma(3.0),
        0.693147180559945309417232121458176568075500134360255254120680,
        1e-14
    );
    assert_almost_eq!(
        ln_gamma(f64::consts::PI),
        0.82769459232343710152957855845235995115350173412073715,
        1e-13
    );
    assert_almost_eq!(
        ln_gamma(3.5),
        1.200973602347074224816021881450712995770238915468157197042113,
        1e-14
    );
    assert_almost_eq!(
        ln_gamma(4.0),
        1.791759469228055000812477358380702272722990692183004705855374,
        1e-14
    );
    assert_almost_eq!(
        ln_gamma(4.5),
        2.453736570842442220504142503435716157331823510689763131380823,
        1e-13
    );
    assert_almost_eq!(
        ln_gamma(5.0 - 1.0e-14),
        3.178053830347930558470257283303394288448414225994179545985931,
        1e-14
    );
    assert_almost_eq!(
        ln_gamma(5.0),
        3.178053830347945619646941601297055408873990960903515214096734,
        1e-14
    );
    assert_almost_eq!(
        ln_gamma(5.0 + 1.0e-14),
        3.178053830347960680823625919312848824873279228348981287761046,
        1e-13
    );
    assert_almost_eq!(
        ln_gamma(5.5),
        3.957813967618716293877400855822590998551304491975006780729532,
        1e-14
    );
    assert_almost_eq!(
        ln_gamma(10.1),
        13.02752673863323795851370097886835481188051062306253294740504,
        1e-14
    );
    assert_almost_eq!(
        ln_gamma(150.0 + 1.0e-12),
        600.0094705553324354062157737572509902987070089159051628001813,
        1e-12
    );
    assert_almost_eq!(
        ln_gamma(1.001e+7),
        1.51342135323817913130119829455205139905331697084416059779e+8,
        1e-13
    );
}
