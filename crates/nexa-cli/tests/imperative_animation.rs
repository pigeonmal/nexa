use nexa_codegen::Backend;

#[test]
fn imperative_float_animation_is_lowered_for_both_native_backends() {
    let module = nexa_compiler::compile(
        r#"app AnimatedProgress {
            state progress: Float64 = 0.0
            body {
                Button("Complete") {
                    withAnimation(Spring(response: 0.35, damping: 0.8)) {
                        progress = 1.0
                    }
                }
                ProgressBar(progress: progress)
            }
        }"#,
    )
    .expect("imperative Float64 animation should compile");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("withAnimation(.spring(response: 0.35, dampingFraction: 0.8)) {"));

    let (kotlin, features) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(features.uses_compose_animation);
    assert!(
        kotlin.contains("nexa_progressAnimationSpec = androidx.compose.animation.core.spring(")
    );
    assert!(kotlin.contains("nexa_progressAnimationSpec by remember"));
    assert!(
        kotlin.contains(
            "nexa_progressAnimated by androidx.compose.animation.core.animateFloatAsState("
        )
    );
    assert!(kotlin.contains("androidx.compose.animation.core.animateFloatAsState("));
    assert!(kotlin.contains("nexa_progressAnimated.toDouble()"));
}
