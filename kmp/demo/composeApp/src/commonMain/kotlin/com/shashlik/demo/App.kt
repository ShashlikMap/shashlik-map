package com.shashlik.demo

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.shashlik.kmp.ShashlikMap
import com.shashlik.kmp.ShashlikMapApiHolder
import org.jetbrains.compose.ui.tooling.preview.Preview
import uniffi.ffi_run.RouteCosting.AUTO
import uniffi.ffi_run.RouteCosting.MOTORBIKE
import uniffi.ffi_run.RouteCosting.PEDESTRIAN
import uniffi.ffi_run.RouteCosting.entries

var routeCosting = mutableStateOf(AUTO)

@Composable
@Preview
fun App() {
    MaterialTheme {
        Box(
            modifier = Modifier.fillMaxSize()
        ) {
            ShashlikMap { x, y->
                ShashlikMapApiHolder.shashlikMapApi?.calculateRoute(x, y, routeCosting.value)
            }
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .align(Alignment.BottomCenter)
                    .background(Color(0, 0, 0, 120))
                    .padding(16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Button(onClick = {
                    routeCosting.value = ((routeCosting.value.ordinal + 1) % entries.size).let {
                        entries[it]
                    }
                }) {
                    when (routeCosting.value) {
                        AUTO -> Text("Auto")
                        PEDESTRIAN -> Text("Pedestrian")
                        MOTORBIKE -> Text("Motorbike")
                    }
                }
                Spacer(modifier = Modifier.width(8.dp))
                Column {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        var checkedState by remember { mutableStateOf(true) }
                        Checkbox(
                            checkedState, onCheckedChange = {
                                ShashlikMapApiHolder.shashlikMapApi?.setCamFollowMode(it)
                                checkedState = it
                            })
                        Text("Camera Mode")
                    }
                    Spacer(modifier = Modifier.height(8.dp))
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        var checkedState by remember { mutableStateOf(false) }
                        Checkbox(
                            checkedState, onCheckedChange = {
                                ShashlikMapApiHolder.shashlikMapApi?.setSsaoMode(it)
                                checkedState = it
                            })
                        Text("SSAO Mode")
                    }
                }

            }
        }
    }
}