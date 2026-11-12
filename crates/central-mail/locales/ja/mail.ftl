# The words of the mails Baylee Central sends (docs/email.md), in Japanese.
#
# One message a line: `id = text`. `{ $name }` is replaced when a mail is written.

brand = Baylee
code-label = コード
code-secret = このコードを他の人に教えないでください。Baylee がコードを尋ねることはありません。
footer = Baylee がメールでパスワードを尋ねることはありません。このメールは、このアドレスが Baylee のアカウントに指定されたため届いています。

# アドレスを確認するコード。
confirm-subject = Baylee のメールアドレスを確認してください
confirm-title = メールアドレスの確認
confirm-intro = このアドレスが Baylee のアカウントに指定されました。確認するには、アドレスを指定した画面でこのコードを入力してください。
confirm-valid = コードの有効期間は{ $minutes }分です。
confirm-ignore = 心当たりがない場合は、このメールを無視してください。コードがなければ、このアドレスは使われません。

# リセットコード。`name` はアカウントのログイン名。
reset-subject = Baylee のリセットコード
reset-title = 新しいパスワードの設定
reset-intro = Baylee のアカウント { $name } のリセットコードが求められました。
reset-how = リセットページでこのコードを入力すると、新しいパスワードを設定できます。
reset-valid = コードは1回だけ、{ $minutes }分間有効です。
reset-link = リセットページを開く
reset-ignore = 心当たりがない場合は、このメールを無視してください。パスワードはそのままです。

# 別のアドレスに置き換えられたアドレスへの通知。
replaced-subject = Baylee のメールアドレスが置き換えられました
replaced-title = このアドレスは置き換えられました
replaced-body = このアドレスは、Baylee のアカウントのメールアドレスではなくなりました。代わりに別のアドレスが確認されました。

# 削除されたアドレスへの通知。
removed-subject = Baylee のメールアドレスが削除されました
removed-title = このアドレスは削除されました
removed-body = このアドレスは、Baylee のアカウントのメールアドレスではなくなりました。アカウントから削除されました。

# 両方の通知。
notice-check = 心当たりがない場合は、ログインして、アカウントのセキュリティページを確認してください。
notice-link = セキュリティページを開く

# The notices to the account's confirmed address that its login changed.
# Not translated yet: English until a translation replaces these lines.
reset-done-subject = The password of your Baylee account was reset
reset-done-title = A new password was set
reset-done-body = A reset code was used to set a new password for the Baylee account of this address.
reset-done-kept = The account's data was kept.
reset-done-new = The account starts with an empty vault. Its data from before cannot be opened any more.
reset-done-sessions = The account is signed out everywhere: every session it had was ended.
reset-done-not-you = If this was not you, someone else can sign in to your account now. Ask a Baylee administrator for a reset code and set your own password again.
reset-done-recovery-kept = If the account has a recovery code, it still opens the data. Keep it: with it, a reset keeps the data.
reset-done-recovery-new = A recovery code from before this reset opens nothing any more.
password-subject = The password of your Baylee account was changed
password-title = The password was changed
password-body = The password of the Baylee account of this address was changed.
password-sessions = Every other session of the account was signed out.
password-not-you = If this was not you, someone else knows your password. Set a new one with a reset code: ask for one on the reset page, or ask a Baylee administrator.
issued-subject = A reset code was made for your Baylee account
issued-title = An administrator made a reset code
issued-body = A Baylee administrator made a reset code for the Baylee account of this address. With it, a new password can be set for the account.
issued-valid = The code works once, for { $hours } hours. Until it is used, your password works as it did.
issued-not-you = If you did not ask for it, sign in and change your password on the security page: that ends the code. Then tell an administrator.
