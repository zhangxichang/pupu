package com.zhangxichang.pupu;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.ParcelFileDescriptor;

public final class FilePicker {
    static {
        System.loadLibrary("app");
    }

    private static final int PICK_FILE_REQUEST = 0x5155;
    private static final int PICK_PICKED = 0;
    private static final int PICK_CANCELED = 1;
    private static final int PICK_FAILED = 2;

    private FilePicker() {
    }

    public static void start(Activity activity, String mimeTypes) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                startOnUiThread(activity, mimeTypes);
            }
        });
    }

    private static void startOnUiThread(Activity activity, String mimeTypes) {
        String[] types = (mimeTypes == null || mimeTypes.isEmpty())
                ? new String[0]
                : mimeTypes.split(",");
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        if (types.length == 0) {
            intent.setType("*/*");
        } else if (types.length == 1) {
            intent.setType(types[0]);
        } else {
            intent.setType("*/*");
            intent.putExtra(Intent.EXTRA_MIME_TYPES, types);
        }
        activity.startActivityForResult(intent, PICK_FILE_REQUEST);
    }

    public static void handleActivityResult(
            Activity activity,
            int requestCode,
            int resultCode,
            Intent data) {
        if (requestCode != PICK_FILE_REQUEST) {
            return;
        }
        if (resultCode != Activity.RESULT_OK || data == null || data.getData() == null) {
            nativeOnFilePicked(PICK_CANCELED, -1);
            return;
        }
        Uri uri = data.getData();
        int fd = openFileDescriptor(activity, uri);
        if (fd < 0) {
            nativeOnFilePicked(PICK_FAILED, -1);
            return;
        }
        nativeOnFilePicked(PICK_PICKED, fd);
    }

    private static int openFileDescriptor(Activity activity, Uri uri) {
        try {
            ParcelFileDescriptor descriptor = activity.getContentResolver().openFileDescriptor(uri, "r");
            if (descriptor == null) {
                return -1;
            }
            return descriptor.detachFd();
        } catch (Exception error) {
            return -1;
        }
    }

    private static native void nativeOnFilePicked(int status, int fd);
}
