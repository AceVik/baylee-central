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

# The notices to the account's confirmed address that its login changed.
reset-done-subject = 你的 Baylee 帳號密碼已被重設
reset-done-title = 已設定新密碼
reset-done-body = 有人用重設碼，為這個地址的 Baylee 帳號設定了新密碼。
reset-done-kept = 帳號的資料已保留。
reset-done-new = 帳號從空保險庫開始。先前的資料已無法再開啟。
reset-done-sessions = 帳號已在所有地方登出：它所有的工作階段都已結束。
reset-done-not-you = 如果這不是你本人操作的，表示別人現在可以登入你的帳號。請向 Baylee 管理員索取重設碼，再設定你自己的密碼。
reset-done-recovery-kept = 如果帳號有救援碼，它仍能打開資料。請保管好：有了它，重設後資料才會保留。
reset-done-recovery-new = 這次重設之前的救援碼，已什麼都打不開。
password-subject = 你的 Baylee 帳號密碼已被變更
password-title = 密碼已變更
password-body = 這個地址的 Baylee 帳號密碼已被變更。
password-sessions = 帳號的其他所有工作階段都已登出。
password-not-you = 如果這不是你本人操作的，表示別人知道你的密碼。請用重設碼設定新的密碼：到重設頁面索取一組，或向 Baylee 管理員索取。
issued-subject = 已為你的 Baylee 帳號產生重設碼
issued-title = 管理員產生了重設碼
issued-body = Baylee 管理員為這個地址的 Baylee 帳號產生了重設碼。有了它，就可以為該帳號設定新密碼。
issued-valid = 這組碼只能使用一次，有效期限為 { $hours } 小時。在使用之前，你的密碼照常有效。
issued-not-you = 如果不是你要求的，請登入並在安全頁面變更你的密碼：這會讓這組碼失效。然後告訴管理員。

# The confirmation mail's button, which opens the security page with the code filled in.
confirm-link = 確認地址

recovery-backup-subject = 你的 Baylee 復原碼備份
recovery-backup-title = 復原碼備份
recovery-backup-intro = 附件包含 Baylee 帳號 { $name } 的復原碼備份。
recovery-backup-encrypted = 壓縮檔 baylee-recovery-code.zip 已使用你設定的密碼加密。
recovery-backup-plain = 壓縮檔 baylee-recovery-code.zip 未設定密碼保護。請在妥善保存後刪除此郵件。
recovery-backup-fact-file = 請將此壓縮檔保存在與此信箱分開的安全位置。
recovery-backup-fact-reset = 任何同時掌握復原碼與此信箱重設權限的人都能開啟你的加密資料。
recovery-backup-warning = 如果不是你本人請求的備份，請忽略此郵件或檢查帳號設定。
recovery-backup-link = 開啟你的帳號
invite-subject = Baylee 好友邀請
invite-title = 好友邀請
invite-intro = { $name } 邀請你在 Baylee 上成為好友。
invite-note-label = 留言：
invite-fact-friend = 接受此一次性連結後，你們將在 Baylee 上成為彼此的好友。
invite-valid-one = 邀請連結有效期限為 1 天。
invite-valid = 邀請連結有效期限為 { $days } 天。
invite-warning = 如果你並未預期收到此邀請，可以忽略這封郵件。
invite-link = 開啟邀請
