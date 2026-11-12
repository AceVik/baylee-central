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
reset-done-subject = Baylee アカウントのパスワードがリセットされました
reset-done-title = 新しいパスワードが設定されました
reset-done-body = このアドレスの Baylee アカウントに、リセットコードで新しいパスワードが設定されました。
reset-done-kept = アカウントのデータは残っています。
reset-done-new = アカウントは空の保管庫から始まります。以前のデータは、もう開けません。
reset-done-sessions = アカウントはすべての場所でログアウトされました。アカウントのすべてのセッションが終了しました。
reset-done-not-you = 心当たりがない場合、ほかの誰かがいまアカウントにログインできます。Baylee の管理者にリセットコードを依頼して、ご自分のパスワードをもう一度設定してください。
reset-done-recovery-kept = アカウントに回復コードがある場合、それでデータを開けます。大切に保管してください。回復コードがあれば、リセットしてもデータが残ります。
reset-done-recovery-new = このリセットより前の回復コードでは、もう何も開けません。
password-subject = Baylee アカウントのパスワードが変更されました
password-title = パスワードが変更されました
password-body = このアドレスの Baylee アカウントのパスワードが変更されました。
password-sessions = アカウントのほかのすべてのセッションがログアウトされました。
password-not-you = 心当たりがない場合、ほかの誰かがあなたのパスワードを知っています。リセットコードで新しいパスワードを設定してください。リセットページでコードを依頼するか、Baylee の管理者に依頼します。
issued-subject = Baylee アカウントのリセットコードが作られました
issued-title = 管理者がリセットコードを作りました
issued-body = Baylee の管理者が、このアドレスの Baylee アカウントのリセットコードを作りました。このコードで、アカウントの新しいパスワードを設定できます。
issued-valid = コードは1回だけ、{ $hours }時間使えます。使われるまでは、パスワードはこれまでどおり使えます。
issued-not-you = 依頼していない場合は、ログインして、セキュリティページでパスワードを変更してください。それでコードは無効になります。そのあと、管理者に知らせてください。

# The confirmation mail's button, which opens the security page with the code filled in.
confirm-link = アドレスを確認
