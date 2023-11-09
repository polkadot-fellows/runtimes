use std::hint::black_box;

use linregress::*;

fn main() {
    let y = vec![1., 2., 3., 4., 5.];
    let x1 = vec![5., 4., 3., 2., 1.];
    let x2 = vec![729.53, 439.0367, 42.054, 1., 0.];
    let x3 = vec![258.589, 616.297, 215.061, 498.361, 0.];
    let data = vec![("Y", y), ("X1", x1), ("X2", x2), ("X3", x3)];
    let data = RegressionDataBuilder::new().build_from(data).unwrap();
    let formula = "Y ~ X1 + X2 + X3";
    tiny_bench::bench_labeled("formula with stats", || {
        FormulaRegressionBuilder::new()
            .data(black_box(&data))
            .formula(black_box(formula))
            .fit()
            .unwrap();
    });
    tiny_bench::bench_labeled("formula without stats", || {
        FormulaRegressionBuilder::new()
            .data(black_box(&data))
            .formula(black_box(formula))
            .fit_without_statistics()
            .unwrap();
    });
    let columns = ("Y", ["X1", "X2", "X3"]);
    tiny_bench::bench_labeled("data columns with stats", || {
        let columns = black_box(columns);
        FormulaRegressionBuilder::new()
            .data(black_box(&data))
            .data_columns(columns.0, columns.1)
            .fit()
            .unwrap();
    });
    tiny_bench::bench_labeled("data columns without stats", || {
        let columns = black_box(columns);
        FormulaRegressionBuilder::new()
            .data(black_box(&data))
            .data_columns(columns.0, columns.1)
            .fit_without_statistics()
            .unwrap();
    });
}
