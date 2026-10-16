# B417 Android menu hang

Cause: the pre-P48 APK hung the Adreno GPU on the indirect-draw path. P48 (direct draws) fixed it, as the post-P48 run2 log shows. The menu used no shadows or MSAA. Phone retest with high settings is in plan/HUMAN.md. → 1-CAUSE-FINAL.md
