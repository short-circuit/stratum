package app.stratum

import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowCompat

class MainActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        WindowCompat.setDecorFitsSystemWindows(window, false)
        super.onCreate(savedInstanceState)
        scheduleSafeAreaInjection()
    }

    private fun scheduleSafeAreaInjection() {
        val handler = Handler(Looper.getMainLooper())
        val inject = object : Runnable {
            override fun run() {
                injectSafeArea()
            }
        }
        handler.postDelayed(inject, 100)
        handler.postDelayed(inject, 500)
        handler.postDelayed(inject, 1500)
    }

    private fun findWebView(): WebView? {
        val content = window.decorView.findViewById<android.view.View>(android.R.id.content)
        if (content is WebView) return content
        if (content is android.view.ViewGroup) {
            for (i in 0 until content.childCount) {
                val c = content.getChildAt(i)
                if (c is WebView) return c
            }
        }
        return null
    }

    private fun injectSafeArea() {
        val insets = ViewCompat.getRootWindowInsets(window.decorView) ?: return
        val wv = findWebView() ?: return
        val sb = insets.getInsets(WindowInsetsCompat.Type.systemBars())
        val topPx = sb.top
        val bottomPx = sb.bottom
        if (topPx <= 0 && bottomPx <= 0) return
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
