package com.shashlik.kmp

/**
 * Marks low-level engine access that is not part of the supported SDK API.
 *
 * It is intended only for the `:shared` module itself and the demo app, and may change or break
 * without notice. Using it requires an explicit compiler option:
 * `-opt-in=com.shashlik.kmp.InternalShashlikMapApi`.
 */
@RequiresOptIn(
    level = RequiresOptIn.Level.ERROR,
    message = "Low-level engine access. Not a supported SDK API; may change or break without notice."
)
@Retention(AnnotationRetention.BINARY)
@Target(AnnotationTarget.CLASS, AnnotationTarget.PROPERTY)
annotation class InternalShashlikMapApi
