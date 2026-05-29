package com.aegis.vault

import android.app.Activity
import android.app.NativeActivity
import android.content.Intent
import android.database.Cursor
import android.net.Uri
import android.os.Bundle
import android.provider.DocumentsContract
import java.io.File
import java.io.FileOutputStream

class MainActivity : NativeActivity() {

    private external fun onFileSelectedNative(path: String)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        instance = this
    }

    fun openFilePicker() {
        runOnUiThread {
            val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "*/*"
            }
            @Suppress("DEPRECATION")
            startActivityForResult(intent, REQUEST_FILE_PICKER)
        }
    }

    fun requestStoragePermission() {
        runOnUiThread {
            try {
                if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
                    if (android.os.Environment.isExternalStorageManager()) {
                        return@runOnUiThread
                    }
                    val uri = Uri.parse("package:$packageName")
                    val intent = Intent(android.provider.Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION, uri)
                    try {
                        startActivity(intent)
                    } catch (e: Exception) {
                        val fallbackIntent = Intent(android.provider.Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION)
                        startActivity(fallbackIntent)
                    }
                } else {
                    val intent = Intent(android.provider.Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
                        data = Uri.parse("package:$packageName")
                    }
                    startActivity(intent)
                }
            } catch (e: Exception) {
                e.printStackTrace()
            }
        }
    }

    @Suppress("DEPRECATION")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == REQUEST_FILE_PICKER) {
            if (resultCode == Activity.RESULT_OK && data != null) {
                val uri: Uri? = data.data
                if (uri != null) {
                    // Berikan URI persisten agar bisa dibaca nanti
                    contentResolver.takePersistableUriPermission(
                        uri,
                        Intent.FLAG_GRANT_READ_URI_PERMISSION
                    )
                    val realPath = getRealPathFromURI(uri) ?: copyToCache(uri) ?: ""
                    onFileSelectedNative(realPath)
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
                if ("primary".equals(type, ignoreCase = true) && split.size > 1) {
                    return android.os.Environment.getExternalStorageDirectory().toString() + "/" + split[1]
                }
            }
        }
        try {
            val cursor: Cursor? = contentResolver.query(uri, arrayOf("_data"), null, null, null)
            cursor?.use {
                if (it.moveToFirst()) {
                    val idx = it.getColumnIndex("_data")
                    if (idx != -1) return it.getString(idx)
                }
            }
        } catch (e: Exception) {
            e.printStackTrace()
        }
        return null
    }

    private fun copyToCache(uri: Uri): String? {
        return try {
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
            FileOutputStream(file).use { out -> inputStream.copyTo(out) }
            inputStream.close()
            file.absolutePath
        } catch (e: Exception) {
            e.printStackTrace()
            null
        }
    }

    companion object {
        private const val REQUEST_FILE_PICKER = 1001

        @Volatile
        private var instance: MainActivity? = null

        @JvmStatic
        fun requestFilePicker() {
            instance?.openFilePicker()
        }

        @JvmStatic
        fun requestStoragePermissionStatic() {
            instance?.requestStoragePermission()
        }

        init {
            System.loadLibrary("aegis_vault")
        }
    }
}
