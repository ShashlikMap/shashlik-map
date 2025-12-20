package com.shashlik.demo2.demo2

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.shashlik.kmp.ShashlikMap
import org.jetbrains.compose.ui.tooling.preview.Preview
import uniffi.ffi_run.RouteCosting

var routeCosting = mutableStateOf(RouteCosting.MOTORBIKE)

@Composable
@Preview
fun App() {
    MaterialTheme {
        Box(
            modifier = Modifier.fillMaxSize()
        ) {
            ShashlikMap()
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .align(Alignment.BottomCenter)
                    .background(Color(0, 0, 0, 150))
                    .padding(16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Button(onClick = {
                    if (routeCosting.value == RouteCosting.MOTORBIKE) {
                        routeCosting.value = RouteCosting.PEDESTRIAN
                    } else {
                        routeCosting.value = RouteCosting.MOTORBIKE
                    }
                }) {
                    if (routeCosting.value == RouteCosting.MOTORBIKE) {
                        Text("Motorbike")
                    } else {
                        Text("Pedestrian")
                    }
                }
                Spacer(modifier = Modifier.width(8.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    var checkedState by remember { mutableStateOf(true) }
                    Checkbox(
                        checkedState, onCheckedChange = {
//                            TempLocationManager.map?.shashlikMapApi?.setCamFollowMode(it)
                            checkedState = it
                        })
                    Text("Camera Mode")
                }
            }
        }
    }
}