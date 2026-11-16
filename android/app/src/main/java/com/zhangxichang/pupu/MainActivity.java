package com.zhangxichang.pupu;

import android.app.NativeActivity;
import android.content.Intent;

public class MainActivity extends NativeActivity {
    public void startFilePicker(String mimeType) {
        FilePicker.start(this, mimeType);
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        FilePicker.handleActivityResult(this, requestCode, resultCode, data);
    }
}
