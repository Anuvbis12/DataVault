package com.aegis.vault

import android.app.Activity
import android.content.Intent
import android.database.Cursor
import android.net.Uri
import android.os.Bundle
import android.provider.MediaStore
import android.provider.DocumentsContract
import android.util.Log
import androidx.activity.result.contract.ActivityResultContracts
import com.google.androidgamesdk.GameActivity
import java.io.File
import java.io.FileOutputStream

class MainActivity : GameActivity() {

    private external fun onFileSelectedNative(uri: String)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        instance = this
    }

    fun openFilePicker() {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "*/*"
        }
        startActivityForResult(intent, 1001)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == 1001) {
            if (resultCode == Activity.RESULT_OK && data != null) {
                val uri: Uri? = data.data
                if (uri != null) {
                    val realPath = getRealPathFromURI(uri)
                    if (realPath != null) {
                        onFileSelectedNative(realPath)
                    } else {
                        // Fallback: copy to cache if unable to resolve
                        val cacheFile = copyToCache(uri)
                        if (cacheFile != null) {
                            onFileSelectedNative(cacheFile)
                        } else {
                            onFileSelectedNative("")
                        }
                    }
                    return
                }
            }
            onFileSelectedNative("")
        }
    }

    private fun getRealPathFromURI(uri: Uri): String? {
        if (DocumentsContract.isDocumentUri(this, uri)) {
            if ("com.android.externalstorage.documents" == uri.authority) {
                val docId = DocumentsContract.getDocumentId(uri)
                val split = docId.split(":")
                val type = split[0]
                if ("primary".equals(type, ignoreCase = true)) {
                    return android.os.Environment.getExternalStorageDirectory().toString() + "/" + split[1]
                }
            }
        }
        // Basic fallback
        var path: String? = null
        try {
            val cursor: Cursor? = contentResolver.query(uri, null, null, null, null)
            cursor?.use {
                if (it.moveToFirst()) {
                    val idx = it.getColumnIndex("_data")
                    if (idx != -1) {
                        path = it.getString(idx)
                    }
                }
            }
        } catch (e: Exception) {
            e.printStackTrace()
        }
        return path
    }

    private fun copyToCache(uri: Uri): String? {
        try {
            val inputStream = contentResolver.openInputStream(uri) ?: return null
            val cursor = contentResolver.query(uri, null, null, null, null)
            var name = "temp_file"
            cursor?.use {
                if (it.moveToFirst()) {
                    val nameIdx = it.getColumnIndex(android.provider.OpenableColumns.DISPLAY_NAME)
                    if (nameIdx != -1) name = it.getString(nameIdx)
                }
            }
            val file = File(cacheDir, name)
            val outputStream = FileOutputStream(file)
            inputStream.copyTo(outputStream)
            inputStream.close()
            outputStream.close()
            return file.absolutePath
        } catch (e: Exception) {
            e.printStackTrace()
            return null
        }
    }

    companion object {
        private var instance: MainActivity? = null

        @JvmStatic
        fun requestFilePicker() {
            instance?.runOnUiThread {
                instance?.openFilePicker()
            }
        }
        
        init {
            System.loadLibrary("aegis_vault")
        }
    }
}
