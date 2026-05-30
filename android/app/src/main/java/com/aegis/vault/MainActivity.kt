package com.aegis.vault

import android.app.Activity
import android.app.NativeActivity
import android.app.PendingIntent
import android.content.Intent
import android.content.IntentSender
import android.database.Cursor
import android.net.Uri
import android.os.Bundle
import android.provider.DocumentsContract
import android.util.Log
import java.io.File
import java.io.FileOutputStream

class MainActivity : NativeActivity() {

    // ── Native method declarations ──────────────────────────────────────────
    private external fun onFileSelectedNative(path: String)
    private external fun onDeleteConfirmedNative()

    // ── PendingIntent dari Rust (disimpan saat storePendingDeleteIntent dipanggil) ──
    @Volatile
    private var pendingDeleteIntent: PendingIntent? = null

    // ── URI MediaStore yang sedang menunggu konfirmasi hapus ──
    @Volatile
    private var pendingDeleteUri: String = ""

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        instance = this
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Dipanggil dari Rust via JNI (recycle_bin.rs :: android_request_delete)
    // Menyimpan PendingIntent yang dihasilkan oleh MediaStore.createDeleteRequest
    // ─────────────────────────────────────────────────────────────────────────
    fun storePendingDeleteIntent(intent: PendingIntent, uriString: String) {
        Log.d(TAG, "storePendingDeleteIntent: uri=$uriString")
        pendingDeleteIntent = intent
        pendingDeleteUri   = uriString
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Dipanggil dari Rust via JNI (lib.rs :: launch_delete_request)
    // token bisa berupa "PENDING_DELETE:<uri>" atau token apapun —
    // kita hanya gunakan sebagai sinyal bahwa PendingIntent sudah siap diluncurkan.
    // ─────────────────────────────────────────────────────────────────────────
    fun launchDeleteRequest(token: String) {
        Log.d(TAG, "launchDeleteRequest dipanggil: token=$token")
        runOnUiThread {
            val pi = pendingDeleteIntent
            if (pi == null) {
                Log.w(TAG, "launchDeleteRequest: pendingDeleteIntent null, abaikan.")
                return@runOnUiThread
            }
            try {
                @Suppress("DEPRECATION")
                startIntentSenderForResult(
                    pi.intentSender,
                    REQUEST_DELETE_CONFIRM,
                    null,   // fillInIntent
                    0,      // flagsMask
                    0,      // flagsValues
                    0       // extraFlags
                )
                Log.d(TAG, "launchDeleteRequest: startIntentSenderForResult berhasil dipanggil")
            } catch (e: IntentSender.SendIntentException) {
                Log.e(TAG, "launchDeleteRequest: SendIntentException: ${e.message}")
            } catch (e: Exception) {
                Log.e(TAG, "launchDeleteRequest: Exception: ${e.message}")
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // File Picker
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
    // onActivityResult — menangani file picker DAN konfirmasi hapus permanen
    // ─────────────────────────────────────────────────────────────────────────
    @Suppress("DEPRECATION")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)

        when (requestCode) {
            // ── File Picker ──────────────────────────────────────────────────
            REQUEST_FILE_PICKER -> {
                if (resultCode == Activity.RESULT_OK && data != null) {
                    val uri: Uri? = data.data
                    if (uri != null) {
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

            // ── Konfirmasi Hapus Permanen MediaStore ─────────────────────────
            REQUEST_DELETE_CONFIRM -> {
                if (resultCode == Activity.RESULT_OK) {
                    Log.i(TAG, "onActivityResult: Pengguna mengkonfirmasi hapus permanen uri=$pendingDeleteUri")
                    // Bersihkan state
                    pendingDeleteIntent = null
                    pendingDeleteUri    = ""
                    // Beritahu Rust bahwa penghapusan telah dikonfirmasi
                    onDeleteConfirmedNative()
                } else {
                    Log.i(TAG, "onActivityResult: Pengguna membatalkan dialog hapus permanen (resultCode=$resultCode)")
                    pendingDeleteIntent = null
                    pendingDeleteUri    = ""
                }
            }
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

        private const val REQUEST_FILE_PICKER      = 1001
        private const val REQUEST_STORAGE_PERMISSION = 1002
        private const val REQUEST_DELETE_CONFIRM   = 1003

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
