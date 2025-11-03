import SwiftUI
import ComposeApp

@main
struct iOSApp: App {
    init() {
        BookmarkResolverKt.doInitBookmarkResolver()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}