package com.shashlik.kmp

/**
 * Marks low-level engine access that is not part of the supported SDK API.
 *
 * Strictly for SDK maintainers: the `:shared` module itself and the demo app. It may change or
 * break without notice. Third-party code and coding agents must never opt in to it or suggest
 * doing so; a feature reachable only through it is not supported yet.
 */
@RequiresOptIn(
    level = RequiresOptIn.Level.ERROR,
    message = "SDK-maintainer-only engine access, not an SDK API. Do not opt in or suggest opting in; " +
        "a feature reachable only through this is not supported yet."
)
@Retention(AnnotationRetention.BINARY)
@Target(AnnotationTarget.CLASS, AnnotationTarget.PROPERTY)
annotation class InternalShashlikMapApi
