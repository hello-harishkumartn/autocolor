# Google Drive setup for AutoColor

AutoColor works without this. Until the three values below are filled in, the Drive button says "Google Drive setup pending" and everything else works. All three values are public (every visitor's browser sees them). None of them is a secret. No secret is ever needed.

AutoColor asks only for the `drive.file` permission. With it the app can read just the files you pick in the Google Picker. Google lists `drive.file` as a non-sensitive scope. Picking a folder does not give access to the files inside it, so in the Picker you open a folder and select the photos in it (Ctrl+A or Cmd+A selects all).

Google renamed parts of the console in 2025: "OAuth consent screen" is now "Google Auth Platform" with the pages Branding, Audience, Data Access and Clients. Both names are given below.

## 1. Create a project

1. Open https://console.cloud.google.com/ and sign in with your Gmail.
2. Click the project picker at the top, then **New project**. Name it `AutoColor` and click **Create**. Select the new project.
3. Open **Project settings** (from the three dot menu on the Dashboard's Project info card) and copy the **Project number** (digits only). This is `GOOGLE_APP_ID`.

## 2. Turn on the two APIs

1. Go to **APIs and Services > Library**.
2. Search for **Google Drive API**, open it and click **Enable**.
3. Search for **Google Picker API**, open it and click **Enable**.

## 3. Consent screen (Google Auth Platform)

1. Go to **APIs and Services > OAuth consent screen** (or **Google Auth Platform**) and click **Get started**.
2. App information: App name `AutoColor`, User support email your Gmail. Click **Next**.
3. Audience: choose **External**. Click **Next**.
4. Contact information: your Gmail. Agree to the policy and click **Create**.
5. **Branding**: App home page `https://harishkumartn.in/projects/autocolor/`, Privacy policy link `https://harishkumartn.in/projects/autocolor/#privacy-title`, Authorised domain `harishkumartn.in`. Save.
6. **Audience**: leave the publishing status on **Testing**. Under **Test users** click **Add users** and add your Gmail. Only test users can sign in while the app is in Testing (up to 100 people).
7. **Data Access**: click **Add or remove scopes**, tick `.../auth/drive.file` ("See, edit, create and delete only the specific Google Drive files you use with this app") and click **Update**, then **Save**. Do not add any other Drive scope.

## 4. OAuth client ID

1. Go to **APIs and Services > Credentials** (or **Google Auth Platform > Clients**) and click **Create credentials > OAuth client ID** (or **Create client**).
2. Application type: **Web application**. Name: `AutoColor web`.
3. **Authorised JavaScript origins**: add `https://harishkumartn.in` and, for testing on your computer, `http://localhost:8765`. Leave **Authorised redirect URIs** empty (AutoColor uses the popup token flow).
4. Click **Create** and copy the **Client ID** (it ends with `.apps.googleusercontent.com`). This is `GOOGLE_CLIENT_ID`. You do not need the client secret. Do not paste it anywhere.

## 5. Browser API key

1. Go to **APIs and Services > Credentials** and click **Create credentials > API key**. Copy the key. This is `GOOGLE_API_KEY`.
2. Click **Edit API key** (or the key's name):
   * **Application restrictions**: **Websites**. Add `https://harishkumartn.in/*` and, while testing, `http://localhost:8765/*`.
   * **API restrictions**: **Restrict key**. Tick **Google Picker API** and **Google Drive API**.
3. Click **Save**. Restrictions can take a few minutes to apply.

## 6. Paste the three values

They go into `config.js`, between the quotes:

| Value | File and line |
|---|---|
| `GOOGLE_CLIENT_ID` | `app/config.js` line 5 (AutoColor repo) and `site/projects/autocolor/config.js` line 5 (site repo) |
| `GOOGLE_API_KEY` | line 7 of the same two files |
| `GOOGLE_APP_ID` (project number) | line 9 of the same two files |

Example: `export const GOOGLE_APP_ID = '123456789012';`

You can also send the three values to the agent that maintains the site and it will paste them for you.

## 7. Test

1. Locally: run `scripts/build.sh`, then `node tests/browser/serve.mjs 8765 /projects/autocolor/=dist` and open `http://localhost:8765/projects/autocolor/`.
2. Click **Choose from Google Drive**. Sign in with your test user Gmail. While the app is in Testing Google shows "Google hasn't verified this app". Click **Continue**.
3. In the Picker open a folder, select photos and click **Select**. The photos are downloaded into the page, corrected and listed. **Download all as zip** saves them.
4. If you see "access blocked" or an origin error, check that the page's address matches an Authorised JavaScript origin exactly (no trailing slash) and that your Gmail is a test user.

## Notes

* Access tokens live only in the page's memory and expire after about an hour. When one expires mid batch the page shows **Reconnect Google Drive**.
* People can remove AutoColor's access at https://myaccount.google.com/permissions.
* Leaving the app in Testing is fine for personal use. Opening it to everyone means publishing the consent screen. With only `drive.file` that should need brand verification rather than a security assessment, but check Google's current rules first.
