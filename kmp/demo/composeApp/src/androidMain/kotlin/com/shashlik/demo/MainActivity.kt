package com.shashlik.demo

import LatLonExtractor
import android.content.Intent
import android.os.Bundle
import android.util.Log
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.lifecycle.lifecycleScope
import com.shashlik.kmp.ShashlikMapApiHolder
import com.shashlik.koordxtract.extractFromIntent
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        setContent {
            App()
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handlingIntent(intent)
    }

    private fun handlingIntent(intent: Intent) {
        lifecycleScope.launch {
            LatLonExtractor().extractFromIntent(intent).let { latLon ->
                Log.d("ShashlikDemo", "Route destination: $latLon")
                latLon.getOrNull()?.let { coord ->
                    ShashlikMapApiHolder.shashlikMapApi?.calculateRouteToLatLon(
                        coord.first,
                        coord.second,
                        routeCosting.value
                    )
                }
            }
        }
    }
}