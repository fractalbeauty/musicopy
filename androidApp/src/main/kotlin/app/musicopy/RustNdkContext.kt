package app.musicopy

import android.content.Context

object RustNdkContext {
    init {
        System.loadLibrary("musicopy")
    }

    external fun init(context: Context)
}
