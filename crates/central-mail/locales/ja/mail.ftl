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
