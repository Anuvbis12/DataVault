package com.aegis.vault

import android.app.Activity
import android.app.NativeActivity
import android.app.PendingIntent
import android.content.Intent
import android.database.Cursor
import android.net.Uri
import android.os.Bundle
import android.provider.DocumentsContract
import android.util.Log
import java.io.File
import java.io.FileOutputStream

/**
 * MainActivity — NativeActivity utama Aegis Vault.
 *
 * Catatan penting: NativeActivity extends android.app.Activity (bukan
 * androidx.activity.ComponentActivity), sehingga registerForActivityResult
 * TIDAK tersedia di sini.
 *
 * Untuk dialog hapus permanen MediaStore digunakan DeleteConfirmActivity
 * (AppCompatActivity transparan) sebagai trampoline agar bisa menggunakan
 * ActivityResultLauncher dengan benar.
 */
class MainActivity : NativeActivity() {

    // ── Native method declarations ──────────────────────────────────────────
    private external fun onFileSelectedNative(path: String)



    // ─────────────────────────────────────────────────────────────────────────
    // File Picker — menggunakan startActivityForResult (deprecated tapi bekerja
    // dengan NativeActivity karena onActivityResult tetap tersedia)
    // ─────────────────────────────────────────────────────────────────────────
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

    // ─────────────────────────────────────────────────────────────────────────
    // Storage Permission
    // ─────────────────────────────────────────────────────────────────────────
    fun requestStoragePermission() {
        runOnUiThread {
            try {
                if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.M) {
                    val permissions = if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.TIRAMISU) {
                        arrayOf(
                            android.Manifest.permission.READ_MEDIA_IMAGES,
                            android.Manifest.permission.READ_MEDIA_VIDEO,
                            android.Manifest.permission.READ_MEDIA_AUDIO
                        )
                    } else {
                        arrayOf(
                            android.Manifest.permission.READ_EXTERNAL_STORAGE,
                            android.Manifest.permission.WRITE_EXTERNAL_STORAGE
                        )
                    }

                    val neededPermissions = permissions.filter {
                        checkSelfPermission(it) != android.content.pm.PackageManager.PERMISSION_GRANTED
                    }

                    if (neededPermissions.isNotEmpty()) {
                        requestPermissions(neededPermissions.toTypedArray(), REQUEST_STORAGE_PERMISSION)
                    } else {
                        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
                            if (!android.os.Environment.isExternalStorageManager()) {
                                val uri = Uri.parse("package:$packageName")
                                val intent = Intent(android.provider.Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION, uri)
                                try {
                                    startActivity(intent)
                                } catch (e: Exception) {
                                    val fallbackIntent = Intent(android.provider.Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION)
                                    startActivity(fallbackIntent)
                                }
                            }
                        }
                    }
                }
            } catch (e: Exception) {
                e.printStackTrace()
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // onActivityResult — hanya menangani file picker
    // (hapus permanen ditangani oleh DeleteConfirmActivity)
    // ─────────────────────────────────────────────────────────────────────────
    @Suppress("DEPRECATION")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)

        if (requestCode == REQUEST_FILE_PICKER) {
            if (resultCode == Activity.RESULT_OK && data != null) {
                val uri: Uri? = data.data
                if (uri != null) {
                    try {
                        contentResolver.takePersistableUriPermission(
                            uri,
                            Intent.FLAG_GRANT_READ_URI_PERMISSION
                        )
                    } catch (e: Exception) {
                        Log.w(TAG, "takePersistableUriPermission gagal: ${e.message}")
                    }
                    val realPath = getRealPathFromURI(uri) ?: copyToCache(uri) ?: ""
                    onFileSelectedNative(realPath)
                    return
                }
            }
            onFileSelectedNative("")
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────────────────────────────────
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
        private const val TAG = "AegisVault"

        private const val REQUEST_FILE_PICKER        = 1001
        private const val REQUEST_STORAGE_PERMISSION = 1002

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
