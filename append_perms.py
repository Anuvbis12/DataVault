with open('Cargo.toml', 'a') as f:
    f.write('\n[[package.metadata.android.uses_permission]]\n')
    f.write('name = "android.permission.READ_EXTERNAL_STORAGE"\n')
    f.write('[[package.metadata.android.uses_permission]]\n')
    f.write('name = "android.permission.WRITE_EXTERNAL_STORAGE"\n')
    f.write('[[package.metadata.android.uses_permission]]\n')
    f.write('name = "android.permission.MANAGE_EXTERNAL_STORAGE"\n')
