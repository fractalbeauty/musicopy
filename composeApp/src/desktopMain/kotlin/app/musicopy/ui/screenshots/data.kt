package app.musicopy.ui.screenshots

import app.musicopy.now
import uniffi.musicopy.CounterModel
import uniffi.musicopy.FileSizeModel
import uniffi.musicopy.IndexItemModel
import uniffi.musicopy.TransferJobModel
import uniffi.musicopy.TransferJobProgressModel

const val demoNodeId = "941117ff675f3ac981ed27eb0bef5f32471bbc493fdc7aa4d416e5fa0d99f83a"

private val sizes = listOf(
    4989763,
    1095414,
    1563412,
    1755442,
    3589461,
    4426223,
    3860711,
    1625446,
    2802457,
    4762734,
    4094873,
    1930753,
    2454012,
    3851429,
    3276907,
    4764122,
    2149031,
    2278378,
    4663917,
    2814759,
    1137473,
    2528498,
    3158457,
    2575123,
    4971571,
    1173993,
    4498963,
    3544791,
    4699278,
    1189320,
    2083823,
    1299363,
    1796448,
    1170094,
    4319501,
    4882643,
    1668526,
    4257289,
    3636606,
    2854346,
    3170149,
    1897216,
    1179902,
    3589414,
    1872222,
    4567303,
    3034815,
    1955498,
    1303603,
    3937282,
    1149949,
    4838527,
    2674153,
    2544626,
    1956673,
    1743934,
    1498660,
    2759534,
    3251613,
    1364076,
    3678021,
    3762847,
    4171143,
    2533240,
    2995016,
    2694362,
    1936234,
    1420893,
    1141532,
    1674254,
    3904120,
    3602940,
    3952412,
    3946779,
    2498419,
    3496822,
    3897814,
    4629084,
    3984871,
    1690893,
    1472701,
    3694391,
    3395602,
    1633864,
    4749040,
    1014299,
    2464815,
    1611962,
    4288876,
    1105184,
    2244933,
    2954970,
    2298109,
    2618941,
    2205969,
    4688319,
    2049593,
    1014674,
    2815484,
    2706566
)

private val fishmonger = listOf(
    "70%",
    "Second hand embarrassment",
    "Bozo bozo bozo",
    "Kinko's field trip 2006",
    "Where did you fall",
    "Spoiled little brat",
    "Your favorite sidekick",
    "Dry land 2001",
    "The fish song",
    "Del mar county fair 2008",
)

private val boneyard = listOf(
    "Everybody's dead!",
    "Girls and boys",
    "Heck",
    "Gunk",
    "Loansharks",
    "Tongue in cheek",
    "Saltfields"
)

val screenshotIndex = buildList {
    var nextSizeIdx = 0
    val nextSize = { FileSizeModel.Actual(sizes[nextSizeIdx++].toULong()) }

    for ((index, title) in boneyard.withIndex()) {
        add(
            IndexItemModel(
                nodeId = demoNodeId,
                root = "Favorites",
                path = "underscores/boneyard/$title.flac",
                fileSize = nextSize(),
                downloaded = false
            )
        )
    }

    add(
        IndexItemModel(
            nodeId = demoNodeId,
            root = "Favorites",
            path = "underscores/Poplife/Poplife.flac",
            fileSize = nextSize(),
            downloaded = true
        )
    )

    repeat(12) {
        add(
            IndexItemModel(
                nodeId = demoNodeId,
                root = "Favorites",
                path = "underscores/Wallsocket/placeholder$it.flac",
                fileSize = nextSize(),
                downloaded = false
            )
        )
    }

    for ((index, title) in fishmonger.withIndex()) {
        add(
            IndexItemModel(
                nodeId = demoNodeId,
                root = "Favorites",
                path = "underscores/fishmonger/$title.flac",
                fileSize = nextSize(),
                downloaded = (index == 1 || index == 4 || index == 6)
            )
        )
    }
}

val screenshotTransferJobs = buildList {
    var nextSizeIdx = 0
    val nextSize = { sizes[nextSizeIdx++].toULong() }
    var nextJobId = 0uL

    // in progress: boneyard 4 5 6
    for (i in listOf(4, 5, 6)) {
        val fileSize = nextSize();
        val fileProgress = (nextSize() % 5uL) * fileSize / 10uL
        add(
            TransferJobModel(
                jobId = nextJobId++,
                fileRoot = "Favorites",
                filePath = "underscores/boneyard/${boneyard[i]}.flac",
                fileSize = fileSize,
                progress = TransferJobProgressModel.InProgress(
                    startedAt = now() - 3uL,
                    bytes = CounterModel(fileProgress)
                )
            )
        )
    }

    // transcoding: boneyard 2 3
    for (i in listOf(2, 3)) {
        add(
            TransferJobModel(
                jobId = nextJobId++,
                fileRoot = "Favorites",
                filePath = "underscores/boneyard/${boneyard[i]}.flac",
                fileSize = nextSize(),
                progress = TransferJobProgressModel.Transcoding
            )
        )
    }

    // finished: fishmonger minus 1 4 6
    for (i in listOf(0, 2, 3, 5, 7, 8, 9)) {
        add(
            TransferJobModel(
                jobId = nextJobId++,
                fileRoot = "Favorites",
                filePath = "underscores/fishmonger/${fishmonger[i]}.flac",
                fileSize = nextSize(),
                progress = TransferJobProgressModel.Finished(
                    finishedAt = now()
                )
            )
        )
    }
    
    // finished: boneyard 0 1
    for (i in listOf(0, 1)) {
        add(
            TransferJobModel(
                jobId = nextJobId++,
                fileRoot = "Favorites",
                filePath = "underscores/boneyard/${boneyard[i]}.flac",
                fileSize = nextSize(),
                progress = TransferJobProgressModel.Finished(
                    finishedAt = now()
                )
            )
        )
    }
}
