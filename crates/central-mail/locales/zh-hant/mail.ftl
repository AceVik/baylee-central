# The words of the mails Baylee Central sends (docs/email.md), in Traditional Chinese.
#
# One message a line: `id = text`. `{ $name }` is replaced when a mail is written.

brand = Baylee
code-label = 你的碼
code-secret = 請勿把這組碼告訴別人。Baylee 的任何人都不會向你索取它。
footer = Baylee 絕不會透過電子郵件向你索取密碼。你收到這封郵件，是因為有人為一個 Baylee 帳號填寫了這個地址。

# The code that confirms an address.
confirm-subject = 確認你在 Baylee 的電子郵件地址
confirm-title = 確認你的電子郵件地址
confirm-intro = 有人為一個 Baylee 帳號填寫了這個地址。要確認它，請在填寫地址的地方輸入這組碼。
confirm-valid = 這組碼的有效期限為 { $minutes } 分鐘。
confirm-ignore = 如果不是你本人操作的，請忽略這封郵件。沒有這組碼，這個地址不會被使用。

# The reset code. `name` is the account's sign-in name.
reset-subject = 你的 Baylee 重設碼
reset-title = 設定新密碼
reset-intro = 有人為 Baylee 帳號 { $name } 要求了一組重設碼。
reset-how = 在重設頁面輸入這組碼，即可設定新密碼。
reset-valid = 這組碼只能使用一次，有效期限為 { $minutes } 分鐘。
reset-link = 開啟重設頁面
reset-ignore = 如果不是你本人操作的，請忽略這封郵件。你的密碼維持不變。

# The notice to an address that another one replaced.
replaced-subject = 你在 Baylee 的電子郵件地址已被取代
replaced-title = 這個地址已被取代
replaced-body = 這個地址不再是某個 Baylee 帳號的電子郵件地址。另一個地址已被確認，取代了它。

# The notice to an address that was removed.
removed-subject = 你在 Baylee 的電子郵件地址已被移除
removed-title = 這個地址已被移除
removed-body = 這個地址不再是某個 Baylee 帳號的電子郵件地址。它已從該帳號中移除。

# Both notices.
notice-check = 如果不是你本人操作的，請登入並查看你帳號的安全頁面。
notice-link = 開啟安全頁面
