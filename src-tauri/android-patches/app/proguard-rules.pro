# Tauri v2 Android — keep WebView bridge and app classes
-keep class app.stratum.** { *; }
-keep class * extends app.stratum.TauriActivity { *; }
-keep class com.google.gson.** { *; }
-keep class org.json.** { *; }
-keep class android.webkit.** { *; }
-keepattributes *Annotation*, JavascriptInterface
-dontwarn com.google.gson.**
-dontwarn org.json.**
