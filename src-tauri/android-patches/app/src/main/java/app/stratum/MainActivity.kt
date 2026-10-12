package app.stratum

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.View
import android.view.ViewGroup
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.enableEdgeToEdge
import androidx.core.view.OnApplyWindowInsetsListener
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
    private var micGranted = false
    private val micLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        micGranted = granted
        findWebView()?.evaluateJavascript(
            "window.__stratumMicPermissionResolve && window.__stratumMicPermissionResolve($granted)",
            null
        )
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        ViewCompat.setOnApplyWindowInsetsListener(window.decorView) { _, insets ->
            injectSafeArea(insets)
            insets
        }
        scheduleSafeAreaInjection()
        scheduleMicBridgeInjection()
    }

    private fun scheduleSafeAreaInjection() {
        val handler = Handler(Looper.getMainLooper())
        val inject = object : Runnable {
            override fun run() {
                val insets = ViewCompat.getRootWindowInsets(window.decorView) ?: return
                injectSafeArea(insets)
            }
        }
        handler.postDelayed(inject, 100)
        handler.postDelayed(inject, 500)
        handler.postDelayed(inject, 1500)
        handler.postDelayed(inject, 5000)
    }

    // Wires the JS->Kotlin microphone bridge once the WebView exists. Mirrors the
    // injectSafeArea retry loop because the WebView is only available after Tauri
    // creates it, which is not guaranteed synchronously in onCreate.
    private fun scheduleMicBridgeInjection() {
        val handler = Handler(Looper.getMainLooper())
        val inject = object : Runnable {
            override fun run() {
                injectMicBridge()
            }
        }
        handler.postDelayed(inject, 100)
        handler.postDelayed(inject, 500)
        handler.postDelayed(inject, 1500)
        handler.postDelayed(inject, 5000)
    }

    private fun injectMicBridge() {
        val wv = findWebView() ?: return
        micGranted = checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED
        wv.addJavascriptInterface(object {
            @JavascriptInterface
            fun isMicGranted(): Boolean = micGranted

            @JavascriptInterface
            fun requestMicPermission(): Boolean {
                if (!micGranted) micLauncher.launch(Manifest.permission.RECORD_AUDIO)
                return micGranted
            }
        }, "StratumMic")
    }

    private fun findWebView(view: View = window.decorView): WebView? {
        if (view is WebView) return view
        if (view is ViewGroup) {
            for (i in 0 until view.childCount) {
                val child = view.getChildAt(i)
                val found = findWebView(child)
                if (found != null) return found
            }
        }
        return null
    }

    private fun injectSafeArea(insets: WindowInsetsCompat) {
        val wv = findWebView() ?: return
        val sb = insets.getInsets(
            WindowInsetsCompat.Type.systemBars() or
            WindowInsetsCompat.Type.displayCutout()
        )
        val topPx = sb.top
        val bottomPx = sb.bottom
        val d = resources.displayMetrics.density
        val topDp = topPx / d
        val bottomDp = bottomPx / d
        val js = "(function(){" +
            "var s=document.documentElement.style;" +
            "s.setProperty('--safe-area-inset-top','${topDp}px');" +
            "s.setProperty('--safe-area-inset-bottom','${bottomDp}px');" +
            "})()"
        wv.evaluateJavascript(js, null)
    }
}
