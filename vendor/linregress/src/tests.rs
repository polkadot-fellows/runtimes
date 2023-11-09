use super::*;

#[test]
fn test_pinv_with_formula_builder() {
    use std::collections::HashMap;
    let inputs = vec![1., 3., 4., 5., 2., 3., 4.];
    let outputs1 = vec![1., 2., 3., 4., 5., 6., 7.];
    let outputs2 = vec![7., 6., 5., 4., 3., 2., 1.];
    let mut data = HashMap::new();
    data.insert("Y", inputs);
    data.insert("X1", outputs1);
    data.insert("X2", outputs2);
    let data = RegressionDataBuilder::new().build_from(data).unwrap();
    let regression = FormulaRegressionBuilder::new()
        .data(&data)
        .formula("Y ~ X1 + X2")
        .fit()
        .expect("Fitting model failed");

    let model_parameters = vec![0.09523809523809523, 0.5059523809523809, 0.2559523809523808];
    let se = vec![
        0.015457637291218289,
        0.1417242813072997,
        0.14172428130729975,
    ];
    let ssr = 9.107142857142858;
    let rsquared = 0.16118421052631582;
    let rsquared_adj = -0.006578947368421018;
    let scale = 1.8214285714285716;
    let pvalues = vec![
        0.001639031204417556,
        0.016044083709847945,
        0.13074580446389245,
    ];
    let residuals = vec![
        -1.392857142857142,
        0.3571428571428581,
        1.1071428571428577,
        1.8571428571428577,
        -1.3928571428571423,
        -0.6428571428571423,
        0.10714285714285765,
    ];
    assert_slices_almost_eq!(regression.parameters(), &model_parameters);
    assert_slices_almost_eq!(regression.se(), &se);
    assert_almost_eq!(regression.ssr(), ssr);
    assert_almost_eq!(regression.rsquared(), rsquared);
    assert_almost_eq!(regression.rsquared_adj(), rsquared_adj);
    assert_slices_almost_eq!(regression.p_values(), &pvalues);
    assert_slices_almost_eq!(regression.residuals(), &residuals);
    assert_almost_eq!(regression.scale(), scale);
}

#[test]
fn test_pinv_with_data_columns() {
    use std::collections::HashMap;
    let inputs = vec![1., 3., 4., 5., 2., 3., 4.];
    let outputs1 = vec![1., 2., 3., 4., 5., 6., 7.];
    let outputs2 = vec![7., 6., 5., 4., 3., 2., 1.];
    let mut data = HashMap::new();
    data.insert("Y", inputs);
    data.insert("X1", outputs1);
    data.insert("X2", outputs2);
    let data = RegressionDataBuilder::new().build_from(data).unwrap();
    let regression = FormulaRegressionBuilder::new()
        .data(&data)
        .data_columns("Y", ["X1", "X2"])
        .fit()
        .expect("Fitting model failed");

    let model_parameters = vec![0.09523809523809523, 0.5059523809523809, 0.2559523809523808];
    let se = vec![
        0.015457637291218289,
        0.1417242813072997,
        0.14172428130729975,
    ];
    let ssr = 9.107142857142858;
    let rsquared = 0.16118421052631582;
    let rsquared_adj = -0.006578947368421018;
    let scale = 1.8214285714285716;
    let pvalues = vec![
        0.001639031204417556,
        0.016044083709847945,
        0.13074580446389245,
    ];
    let residuals = vec![
        -1.392857142857142,
        0.3571428571428581,
        1.1071428571428577,
        1.8571428571428577,
        -1.3928571428571423,
        -0.6428571428571423,
        0.10714285714285765,
    ];
    assert_slices_almost_eq!(regression.parameters(), &model_parameters);
    assert_slices_almost_eq!(regression.se(), &se);
    assert_almost_eq!(regression.ssr(), ssr);
    assert_almost_eq!(regression.rsquared(), rsquared);
    assert_almost_eq!(regression.rsquared_adj(), rsquared_adj);
    assert_slices_almost_eq!(regression.p_values(), &pvalues);
    assert_slices_almost_eq!(regression.residuals(), &residuals);
    assert_almost_eq!(regression.scale(), scale);
}

#[test]
fn test_regression_standard_error_equal_to_zero_does_not_prevent_fitting() {
    // Regression test for underlying issue of https://github.com/n1m3/linregress/issues/9

    // The following input does not conform to our API (we expect that all intercepts == 1, not 0),
    // but Hyrum's law...
    let data = vec![
        0.0,
        0.0,
        0.0,
        34059798.0,
        0.0,
        1.0,
        66771421.0,
        0.0,
        2.0,
        100206133.0,
        0.0,
        3.0,
        133435943.0,
        0.0,
        4.0,
        166028256.0,
        0.0,
        5.0,
        199723152.0,
        0.0,
        6.0,
        233754352.0,
        0.0,
        7.0,
        267284084.0,
        0.0,
        8.0,
        301756656.0,
        0.0,
        9.0,
        331420366.0,
        0.0,
        10.0,
        367961084.0,
        0.0,
        11.0,
        401288216.0,
        0.0,
        12.0,
        434555574.0,
        0.0,
        13.0,
        469093436.0,
        0.0,
        14.0,
        501541551.0,
        0.0,
        15.0,
        523986797.0,
        0.0,
        16.0,
        558792615.0,
        0.0,
        17.0,
        631494010.0,
        0.0,
        18.0,
        669229109.0,
        0.0,
        19.0,
        704321427.0,
        0.0,
        20.0,
    ];
    let rows = 21;
    let columns = 3;
    fit_low_level_regression_model(&data, rows, columns).unwrap();
}

#[test]
fn test_low_level_model_fitting() {
    let inputs = vec![1., 3., 4., 5., 2., 3., 4.];
    let outputs1 = vec![1., 2., 3., 4., 5., 6., 7.];
    let outputs2 = vec![7., 6., 5., 4., 3., 2., 1.];
    let mut data_row_major = Vec::with_capacity(4 * 7);
    for n in 0..7 {
        data_row_major.push(inputs[n]);
        data_row_major.push(1.0);
        data_row_major.push(outputs1[n]);
        data_row_major.push(outputs2[n]);
    }
    let regression = fit_low_level_regression_model(&data_row_major, 7, 4).unwrap();
    let model_parameters = vec![0.09523809523809523, 0.5059523809523809, 0.2559523809523808];
    let se = vec![
        0.015457637291218289,
        0.1417242813072997,
        0.14172428130729975,
    ];
    let ssr = 9.107142857142858;
    let rsquared = 0.16118421052631582;
    let rsquared_adj = -0.006578947368421018;
    let scale = 1.8214285714285716;
    let pvalues = vec![
        0.001639031204417556,
        0.016044083709847945,
        0.13074580446389245,
    ];
    let residuals = vec![
        -1.392857142857142,
        0.3571428571428581,
        1.1071428571428577,
        1.8571428571428577,
        -1.3928571428571423,
        -0.6428571428571423,
        0.10714285714285765,
    ];
    assert_slices_almost_eq!(regression.parameters(), &model_parameters);
    assert_slices_almost_eq!(regression.se(), &se);
    assert_almost_eq!(regression.ssr(), ssr);
    assert_almost_eq!(regression.rsquared(), rsquared);
    assert_almost_eq!(regression.rsquared_adj(), rsquared_adj);
    assert_slices_almost_eq!(regression.p_values(), &pvalues);
    assert_slices_almost_eq!(regression.residuals(), &residuals);
    assert_almost_eq!(regression.scale(), scale);
}

#[test]
fn test_without_statistics() {
    use std::collections::HashMap;
    let inputs = vec![1., 3., 4., 5., 2., 3., 4.];
    let outputs1 = vec![1., 2., 3., 4., 5., 6., 7.];
    let outputs2 = vec![7., 6., 5., 4., 3., 2., 1.];
    let mut data = HashMap::new();
    data.insert("Y", inputs);
    data.insert("X1", outputs1);
    data.insert("X2", outputs2);
    let data = RegressionDataBuilder::new().build_from(data).unwrap();
    let regression = FormulaRegressionBuilder::new()
        .data(&data)
        .formula("Y ~ X1 + X2")
        .fit_without_statistics()
        .expect("Fitting model failed");
    let model_parameters = vec![0.09523809523809523, 0.5059523809523809, 0.2559523809523808];
    assert_slices_almost_eq!(&regression, &model_parameters);
}

#[test]
fn test_invalid_input_empty_matrix() {
    let y = vec![];
    let x1 = vec![];
    let x2 = vec![];
    let data = vec![("Y", y), ("X1", x1), ("X2", x2)];
    let data = RegressionDataBuilder::new().build_from(data);
    assert!(data.is_err());
}

#[test]
fn test_invalid_input_wrong_shape_x() {
    let y = vec![1., 2., 3.];
    let x1 = vec![1., 2., 3.];
    let x2 = vec![1., 2.];
    let data = vec![("Y", y), ("X1", x1), ("X2", x2)];
    let data = RegressionDataBuilder::new().build_from(data);
    assert!(data.is_err());
}

#[test]
fn test_invalid_input_wrong_shape_y() {
    let y = vec![1., 2., 3., 4.];
    let x1 = vec![1., 2., 3.];
    let x2 = vec![1., 2., 3.];
    let data = vec![("Y", y), ("X1", x1), ("X2", x2)];
    let data = RegressionDataBuilder::new().build_from(data);
    assert!(data.is_err());
}

#[test]
fn test_invalid_input_nan() {
    let y1 = vec![1., 2., 3., 4.];
    let x1 = vec![1., 2., 3., std::f64::NAN];
    let data1 = vec![("Y", y1), ("X", x1)];
    let y2 = vec![1., 2., 3., std::f64::NAN];
    let x2 = vec![1., 2., 3., 4.];
    let data2 = vec![("Y", y2), ("X", x2)];
    let r_data1 = RegressionDataBuilder::new().build_from(data1.to_owned());
    let r_data2 = RegressionDataBuilder::new().build_from(data2.to_owned());
    assert!(r_data1.is_err());
    assert!(r_data2.is_err());
    let builder = RegressionDataBuilder::new();
    let builder = builder.invalid_value_handling(InvalidValueHandling::DropInvalid);
    let r_data1 = builder.build_from(data1);
    let r_data2 = builder.build_from(data2);
    assert!(r_data1.is_ok());
    assert!(r_data2.is_ok());
}

#[test]
fn test_invalid_input_infinity() {
    let y1 = vec![1., 2., 3., 4.];
    let x1 = vec![1., 2., 3., std::f64::INFINITY];
    let data1 = vec![("Y", y1), ("X", x1)];
    let y2 = vec![1., 2., 3., std::f64::NEG_INFINITY];
    let x2 = vec![1., 2., 3., 4.];
    let data2 = vec![("Y", y2), ("X", x2)];
    let r_data1 = RegressionDataBuilder::new().build_from(data1.to_owned());
    let r_data2 = RegressionDataBuilder::new().build_from(data2.to_owned());
    assert!(r_data1.is_err());
    assert!(r_data2.is_err());
    let builder = RegressionDataBuilder::new();
    let builder = builder.invalid_value_handling(InvalidValueHandling::DropInvalid);
    let r_data1 = builder.build_from(data1);
    let r_data2 = builder.build_from(data2);
    assert!(r_data1.is_ok());
    assert!(r_data2.is_ok());
}

#[test]
fn test_invalid_input_all_equal_columns() {
    let y = vec![38.0, 38.0, 38.0];
    let x = vec![42.0, 42.0, 42.0];
    let data = vec![("y", y), ("x", x)];
    let data = RegressionDataBuilder::new().build_from(data);
    assert!(data.is_err());
}

#[test]
fn test_drop_invalid_values() {
    let mut data: HashMap<Cow<'_, str>, Vec<f64>> = HashMap::new();
    data.insert("Y".into(), vec![-1., -2., -3., -4.]);
    data.insert("foo".into(), vec![1., 2., 12., 4.]);
    data.insert("bar".into(), vec![1., 1., 7., 4.]);
    data.insert("baz".into(), vec![1.3333, 2.754, 3.12, 4.11]);
    assert_eq!(RegressionData::drop_invalid_values(data.to_owned()), data);
    data.insert(
        "invalid".into(),
        vec![std::f64::NAN, 42., std::f64::NEG_INFINITY, 23.11],
    );
    data.insert(
        "invalid2".into(),
        vec![1.337, -3.14, std::f64::INFINITY, 11.111111],
    );
    let mut ref_data: HashMap<Cow<'_, str>, Vec<f64>> = HashMap::new();
    ref_data.insert("Y".into(), vec![-2., -4.]);
    ref_data.insert("foo".into(), vec![2., 4.]);
    ref_data.insert("bar".into(), vec![1., 4.]);
    ref_data.insert("baz".into(), vec![2.754, 4.11]);
    ref_data.insert("invalid".into(), vec![42., 23.11]);
    ref_data.insert("invalid2".into(), vec![-3.14, 11.111111]);
    assert_eq!(
        ref_data,
        RegressionData::drop_invalid_values(data.to_owned())
    );
}

#[test]
fn test_all_invalid_input() {
    let data = vec![
        ("Y", vec![1., 2., 3.]),
        ("X", vec![std::f64::NAN, std::f64::NAN, std::f64::NAN]),
    ];
    let builder = RegressionDataBuilder::new();
    let builder = builder.invalid_value_handling(InvalidValueHandling::DropInvalid);
    let r_data = builder.build_from(data);
    assert!(r_data.is_err());
}

#[test]
fn test_invalid_column_names() {
    let data1 = vec![("x~f", vec![1., 2., 3.]), ("foo", vec![0., 0., 0.])];
    let data2 = vec![("foo", vec![1., 2., 3.]), ("foo+", vec![0., 0., 0.])];
    let builder = RegressionDataBuilder::new();
    assert!(builder.build_from(data1).is_err());
    assert!(builder.build_from(data2).is_err());
}

#[test]
fn test_no_formula() {
    let data = vec![("x", vec![1., 2., 3.]), ("foo", vec![0., 0., 0.])];
    let data = RegressionDataBuilder::new().build_from(data).unwrap();
    let res = FormulaRegressionBuilder::new().data(&data).fit();
    assert!(res.is_err());
}

#[test]
fn test_both_formula_and_data_columns() {
    let y = vec![1., 2., 3., 4., 5.];
    let x1 = vec![5., 4., 3., 2., 1.];
    let x2 = vec![729.53, 439.0367, 42.054, 1., 0.];
    let x3 = vec![258.589, 616.297, 215.061, 498.361, 0.];
    let data = vec![("Y", y), ("X1", x1), ("X2", x2), ("X3", x3)];
    let data = RegressionDataBuilder::new().build_from(data).unwrap();
    let formula = "Y ~ X1 + X2 + X3";
    let res = FormulaRegressionBuilder::new()
        .data(&data)
        .formula(formula)
        .data_columns("Y", ["X1", "X2", "X3"])
        .fit();
    assert!(res.is_err());
}

fn build_model() -> RegressionModel {
    let y = vec![1., 2., 3., 4., 5.];
    let x1 = vec![5., 4., 3., 2., 1.];
    let x2 = vec![729.53, 439.0367, 42.054, 1., 0.];
    let x3 = vec![258.589, 616.297, 215.061, 498.361, 0.];
    let data = vec![("Y", y), ("X1", x1), ("X2", x2), ("X3", x3)];
    let data = RegressionDataBuilder::new().build_from(data).unwrap();
    let formula = "Y ~ X1 + X2 + X3";
    FormulaRegressionBuilder::new()
        .data(&data)
        .formula(formula)
        .fit()
        .unwrap()
}

#[test]
fn test_prediction_empty_vectors() {
    let model = build_model();
    let new_data: HashMap<Cow<'_, _>, _> = vec![("X1", vec![]), ("X2", vec![]), ("X3", vec![])]
        .into_iter()
        .map(|(x, y)| (Cow::from(x), y))
        .collect();
    assert!(model.check_variables(&new_data).is_err());
}

#[test]
fn test_prediction_vectors_with_different_lengths() {
    let model = build_model();
    let new_data: HashMap<Cow<'_, _>, _> = vec![
        ("X1", vec![1.0, 2.0]),
        ("X2", vec![2.0, 1.0]),
        ("X3", vec![3.0]),
    ]
    .into_iter()
    .map(|(x, y)| (Cow::from(x), y))
    .collect();
    assert!(model.check_variables(&new_data).is_err());
}

#[test]
fn test_too_many_prediction_variables() {
    let model = build_model();
    let new_data: HashMap<Cow<'_, _>, _> = vec![
        ("X1", vec![1.0]),
        ("X2", vec![2.0]),
        ("X3", vec![3.0]),
        ("X4", vec![4.0]),
    ]
    .into_iter()
    .map(|(x, y)| (Cow::from(x), y))
    .collect();
    assert!(model.check_variables(&new_data).is_err());
}

#[test]
fn test_not_enough_prediction_variables() {
    let model = build_model();
    let new_data: HashMap<Cow<'_, _>, _> = vec![("X1", vec![1.0]), ("X2", vec![2.0])]
        .into_iter()
        .map(|(x, y)| (Cow::from(x), y))
        .collect();
    assert!(model.check_variables(&new_data).is_err());
}

#[test]
fn test_prediction() {
    let model = build_model();
    let new_data = vec![("X1", vec![2.5]), ("X2", vec![2.0]), ("X3", vec![2.0])];
    let prediction = model.predict(new_data).unwrap();
    assert_eq!(prediction.len(), 1);
    assert_almost_eq!(prediction[0], 3.500000000000111, 1.0E-7);
}

#[test]
fn test_multiple_predictions() {
    let model = build_model();
    let new_data = vec![
        ("X1", vec![2.5, 3.5]),
        ("X2", vec![2.0, 8.0]),
        ("X3", vec![2.0, 1.0]),
    ];
    let prediction = model.predict(new_data).unwrap();
    assert_eq!(prediction.len(), 2);
    assert_almost_eq!(prediction[0], 3.500000000000111, 1.0E-7);
    assert_almost_eq!(prediction[1], 2.5000000000001337, 1.0E-7);
}

#[test]
fn test_multiple_predictions_out_of_order() {
    let model = build_model();
    let new_data = vec![
        ("X1", vec![2.5, 3.5]),
        ("X3", vec![2.0, 1.0]),
        ("X2", vec![2.0, 8.0]),
    ];
    let prediction = model.predict(new_data).unwrap();
    assert_eq!(prediction.len(), 2);
    assert_almost_eq!(prediction[0], 3.500000000000111, 1.0E-7);
    assert_almost_eq!(prediction[1], 2.5000000000001337, 1.0E-7);
}
