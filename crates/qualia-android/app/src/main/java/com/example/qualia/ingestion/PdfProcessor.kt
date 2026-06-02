package com.example.qualia.ingestion

import android.content.Context
import android.graphics.Bitmap
import android.graphics.pdf.PdfRenderer
import android.net.Uri
import android.os.ParcelFileDescriptor
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Replaces the legacy Python `pypdf` logic. 
 * Renders PDF pages to bitmaps which can then be fed into the Vision-Language Model
 * (e.g., Phi-3.5-Vision via ONNX) for localized OCR and structured extraction.
 */
object PdfProcessor {

    suspend fun processPdfToBitmaps(context: Context, uri: Uri): List<Bitmap> {
        return withContext(Dispatchers.IO) {
            val bitmaps = mutableListOf<Bitmap>()
            try {
                val pfd = context.contentResolver.openFileDescriptor(uri, "r")
                if (pfd != null) {
                    val renderer = PdfRenderer(pfd)
                    for (i in 0 until renderer.pageCount) {
                        val page = renderer.openPage(i)
                        
                        // We render at 2x density for better OCR resolution by the VLM
                        val bitmap = Bitmap.createBitmap(
                            page.width * 2,
                            page.height * 2,
                            Bitmap.Config.ARGB_8888
                        )
                        
                        // White background
                        bitmap.eraseColor(android.graphics.Color.WHITE)
                        
                        page.render(bitmap, null, null, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
                        bitmaps.add(bitmap)
                        page.close()
                    }
                    renderer.close()
                    pfd.close()
                }
            } catch (e: Exception) {
                e.printStackTrace()
            }
            bitmaps
        }
    }
}
