use super::*;

pub(super) fn arch() -> Logo {
    let lines = vec![
        r##"                  -`"##,
        r##"                 .o+`"##,
        r##"                `ooo/"##,
        r##"               `+oooo:"##,
        r##"              `+oooooo:"##,
        r##"              -+oooooo+:"##,
        r##"            `/:-:++oooo+:"##,
        r##"           `/++++/+++++++:"##,
        r##"          `/++++++++++++++:"##,
        r##"         `/+++o$2oooooooo$1oooo/`"##,
        r##"        ./$2ooosssso++osssssso$1+`"##,
        r##"$2       .oossssso-````/ossssss+`"##,
        r##"      -osssssso.      :ssssssso."##,
        r##"     :osssssss/        osssso+++."##,
        r##"    /ossssssss/        +ssssooo/-"##,
        r##"  `/ossssso+/:-        -:/+osssso+-"##,
        r##" `+sso+:-`                 `.-/+oso:"##,
        r##"`++:.                           `-/+/"##,
        r##".`                                 `/"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: ARCH_CYAN,
        colors: vec![ARCH_CYAN, ARCH_LIGHT],
        width,
    }
}

pub(super) fn manjaro() -> Logo {
    let lines = vec![
        r##"██████████████████  ████████"##,
        r##"██████████████████  ████████"##,
        r##"██████████████████  ████████"##,
        r##"██████████████████  ████████"##,
        r##"████████            ████████"##,
        r##"████████  ████████  ████████"##,
        r##"████████  ████████  ████████"##,
        r##"████████  ████████  ████████"##,
        r##"████████  ████████  ████████"##,
        r##"████████  ████████  ████████"##,
        r##"████████  ████████  ████████"##,
        r##"████████  ████████  ████████"##,
        r##"████████  ████████  ████████"##,
        r##"████████  ████████  ████████"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: MANJARO_GREEN,
        colors: vec![MANJARO_GREEN],
        width,
    }
}

pub(super) fn endeavour() -> Logo {
    let lines = vec![
        r##"                     $2./$1o$3."##,
        r##"                   $2./$1sssso$3-"##,
        r##"                 $2`:$1osssssss+$3-"##,
        r##"               $2`:+$1sssssssssso$3/."##,
        r##"             $2`-/o$1ssssssssssssso$3/."##,
        r##"           $2`-/+$1sssssssssssssssso$3+:`"##,
        r##"         $2`-:/+$1sssssssssssssssssso$3+/."##,
        r##"       $2`.://o$1sssssssssssssssssssso$3++-"##,
        r##"      $2.://+$1ssssssssssssssssssssssso$3++:"##,
        r##"    $2.:///o$1ssssssssssssssssssssssssso$3++:"##,
        r##"  $2`:////$1ssssssssssssssssssssssssssso$3+++."##,
        r##"$2`-////+$1ssssssssssssssssssssssssssso$3++++-"##,
        r##" $2`..-+$1oosssssssssssssssssssssssso$3+++++/`"##,
        r##"   $3./++++++++++++++++++++++++++++++/:."##,
        r##"  `:::::::::::::::::::::::::------``"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: ENDEAVOUR_PURPLE,
        colors: vec![ENDEAVOUR_PURPLE, ENDEAVOUR_RED, ENDEAVOUR_BLUE],
        width,
    }
}

pub(super) fn steamos() -> Logo {
    let lines = vec![
        r##"$1              .,,,,."##,
        r##"        .,'onNMMMMMNNnn',."##,
        r##"     .'oNMANKMMMMMMMMMMMNNn'."##,
        r##"   .'ANMMMMMMMXKNNWWWPFFWNNMNn."##,
        r##"  ;NNMMMMMMMMMMNWW'' ,.., 'WMMM,"##,
        r##" ;NMMMMV+##+VNWWW' .+;'':+, 'WMW,"##,
        r##",VNNWP+$2######$1+WW,  $2+:    $1:+, +MMM,"##,
        r##"'$2+#############,   +.    ,+' $1+NMMM"##,
        r##"$2  '*#########*'     '*,,*' $1.+NMMMM."##,
        r##"$2     `'*###*'          ,.,;###$1+WNM,"##,
        r##"$2         .,;;,      .;##########$1+W"##,
        r##"$2,',.         ';  ,+##############'"##,
        r##" '###+. :,. .,; ,###############'"##,
        r##"  '####.. `'' .,###############'"##,
        r##"    '#####+++################'"##,
        r##"      '*##################*'"##,
        r##"         ''*##########*''"##,
        r##"              ''''''"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: STEAM_BLUE,
        colors: vec![STEAM_BLUE, WHITE],
        width,
    }
}

pub(super) fn artix() -> Logo {
    let lines = vec![
        r##"                   '"##,
        r##"                  'o'"##,
        r##"                 'ooo'"##,
        r##"                'ooxoo'"##,
        r##"               'ooxxxoo'"##,
        r##"              'oookkxxoo'"##,
        r##"             'oiioxkkxxoo'"##,
        r##"            ':;:iiiioxxxoo'"##,
        r##"               `'.;::ioxxoo'"##,
        r##"          '-.      `':;jiooo'"##,
        r##"         'oooio-..     `'i:io'"##,
        r##"        'ooooxxxxoio:,.   `'-;'"##,
        r##"       'ooooxxxxxkkxoooIi:-.  `'"##,
        r##"      'ooooxxxxxkkkkxoiiiiiji'"##,
        r##"     'ooooxxxxxkxxoiiii:'`     .i'"##,
        r##"    'ooooxxxxxoi:::'`       .;ioxo'"##,
        r##"   'ooooxooi::'`         .:iiixkxxo'"##,
        r##"  'ooooi:'`                `'';ioxxo'"##,
        r##" 'i:'`                          '':io'"##,
        r##"'`                                   `'"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: ARTIX_CYAN,
        colors: vec![ARTIX_CYAN],
        width,
    }
}

pub(super) fn cachyos() -> Logo {
    let lines = vec![
        r##"           $3.$1-------------------------:"##,
        r##"          .$2+=$1========================."##,
        r##"         :$2++$1===$2++===$1===============-       :$2++$1-"##,
        r##"        :$2*++$1====$2+++++==$1===========-        .==:"##,
        r##"       -$2*+++$1=====$2+***++=$1=========:"##,
        r##"      =$2*++++=$1=======------------:"##,
        r##"     =$2*+++++=$1====-                     $3...$1"##,
        r##"   .$2+*+++++$1=-===:                    .$2=+++=$1:"##,
        r##"  :$2++++$1=====-==:                     -***$2**$1+"##,
        r##" :$2++=$1=======-=.                      .=+**+$3.$1"##,
        r##".$2+$1==========-.                          $3.$1"##,
        r##" :$2+++++++$1====-                                $3.$1--==-$3.$1"##,
        r##"  :$2++$1==========.                             $3:$2+++++++$1$3:"##,
        r##"   $1.-===========.                            =*****+*+"##,
        r##"    $1.-===========:                           .+*****+:"##,
        r##"      $1-=======$2++++$1:::::::::::::::::::::::::-:  $3.$1---:"##,
        r##"       :======$2++++$1====$2+++******************=."##,
        r##"        $1:=====$2+++$1==========$2++++++++++++++*-"##,
        r##"         $1.====$2++$1==============$2++++++++++*-"##,
        r##"          $1.===$2+$1==================$2+++++++:"##,
        r##"           $1.-=======================$2+++:"##,
        r##"             $3.........................."##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: CACHYOS_CYAN,
        colors: vec![CACHYOS_CYAN, CACHYOS_GREEN, CACHYOS_DARK],
        width,
    }
}

pub(super) fn cachyos_small() -> Logo {
    let lines = vec![
        r##"     ..........."##,
        r##"    $2/,,,,$1...../"##,
        r##"   $2/,,,$1..$2,,$1../   $2()"##,
        r##"  $2/,,,,$1../"##,
        r##" $2/,,,$1.../     $1/'\"##,
        r##"$2/,$1...../      $2\,/"##,
        r##"$2\,,,,,,\\             $1_"##,
        r##" $2\,,$1....\\           $1/ \"##,
        r##"  $1\...$2,$1..\\          $2\_/"##,
        r##"   $1\..$2,,,,,,,,,,,,/"##,
        r##"    $1\$2,,$1........$2,,/"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: CACHYOS_CYAN,
        colors: vec![CACHYOS_GREEN, CACHYOS_CYAN],
        width,
    }
}

pub(super) fn arch_small() -> Logo {
    let lines = vec![
        r##"      /\"##,
        r##"     /  \"##,
        r##"    /    \"##,
        r##"   /      \"##,
        r##"$2  /   ,,   \"##,
        r##" /   |  |   \"##,
        r##"/_-''    ''-_\"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: ARCH_CYAN,
        colors: vec![ARCH_CYAN, ARCH_LIGHT],
        width,
    }
}

pub(super) fn garuda() -> Logo {
    let lines = vec![
        r##"                   .%;888:8898898:"##,
        r##"                 x;XxXB%89b8:b8%b88:"##,
        r##"              .8Xxd                8X:."##,
        r##"            .8Xx;                    8x:."##,
        r##"          .tt8x          .d            x88;"##,
        r##"       .@8x8;          .db:              xx@;"##,
        r##"     ,tSXX°          .bbbbbbbbbbbbbbbbbbbB8x@;"##,
        r##"   .SXxx            bBBBBBBBBBBBBBBBBBBBbSBX8;"##,
        r##" ,888S                                     pd!"##,
        r##"8X88/                                       q"##,
        r##"8X88/"##,
        r##"GBB."##,
        r##" x%88        d888@8@X@X@X88X@@XX@@X@8@X."##,
        r##"   dxXd    dB8b8b8B8B08bB88b998888b88x."##,
        r##"    dxx8o                      .@@;."##,
        r##"      dx88                   .t@x."##,
        r##"        d:SS@8ba89aa67a853Sxxad."##,
        r##"          .d988999889889899dd."##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: GARUDA_ORANGE,
        colors: vec![GARUDA_ORANGE, GARUDA_YELLOW],
        width,
    }
}

pub(super) fn archcraft() -> Logo {
    let lines = vec![
        r##"$1⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⢰⡆$1⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄"##,
        r##"$2⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⢠⣿⣿⡄$2⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄"##,
        r##"$3⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⢀⣾⣿⣿⣿⡀$3⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄"##,
        r##"$4⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⣼⣿⣿⣿⣿⣷⡀$4⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄"##,
        r##"$5⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⣼⣿⣿⣿⣿⣿⣿⣷$5⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄"##,
        r##"$6⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⢼⣿⣿⣿⣿⣿⣿⣿⣿⣧$6⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄"##,
        r##"$1⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⣰⣤⣈⠻⢿⣿⣿⣿⣿⣿⣿⣧$1⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄"##,
        r##"$2⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⣰⣿⣿⣿⣿⣮⣿⣿⣿⣿⣿⣿⣿⣧$2⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄"##,
        r##"$3⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⣰⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣧$3⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄"##,
        r##"$4⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⣰⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣧$4⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄"##,
        r##"$5⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⣼⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣧$5⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄"##,
        r##"$6⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⣼⣿⣿⣿⣿⣿⡿⣿⣿⡟$6⠄⠄$1⠸⣿⣿⡿⣿⣿⣿⣿⣿⣷⡀$6⠄⠄⠄⠄⠄⠄⠄⠄"##,
        r##"$1⠄⠄⠄⠄⠄⠄⠄⠄$1⣼⣿⣿⣿⣿⣿⡏$1⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⠈⣿⣿⣿⣿⣿⣷⡀$1⠄⠄⠄⠄⠄⠄⠄"##,
        r##"$2⠄⠄⠄⠄⠄⠄$1⢀⣼⣿⣿⣿⣿⣿⣿⡗$2⠄⠄⠄$1⢀⣠⣤⣀⠄⠄⠄$1⠸⣿⣿⣿⣿⣿⣿⣷⡀$2⠄⠄⠄⠄⠄⠄"##,
        r##"$3⠄⠄⠄⠄⠄$1⢀⣾⣿⣿⣿⣿⣿⡏⠁$3⠄⠄⠄$1⢠⣿⣿⣿⣿⡇$3⠄⠄⠄⠄$1⢙⣿⣿⣻⠿⣿⣷⡀$3⠄⠄⠄⠄⠄"##,
        r##"$4⠄⠄⠄⠄$1⢀⣾⣿⣿⣿⣿⣿⣿⣷⣤⡀$4⠄⠄⠄$1⠻⣿⣿⡿⠃$4⠄⠄⠄$1⢀⣼⣿⣿⣿⣿⣦⣌⠙⠄$4⠄⠄⠄⠄"##,
        r##"$5⠄⠄⠄$1⢠⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⠏$5⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⢿⣿⣿⣿⣿⣿⣿⣿⣿⣦⡀$5⠄⠄⠄"##,
        r##"$6⠄⠄$1⢠⣿⣿⣿⣿⣿⣿⣿⡿⠟⠋⠁$6⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⠙⠻⣿⣿⣿⣿⣿⣿⣿⣿⡄$6⠄⠄"##,
        r##"$1⠄$1⣠⣿⣿⣿⣿⠿⠛⠋⠁$1⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⠉⠙⠻⢿⣿⣿⣿⣿⣆$1⠄"##,
        r##"$1⡰⠟⠛⠉⠁$2⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄$1⠉⠙⠛⠿⢆"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: ARCHCRAFT_TEAL,
        colors: vec![
            ARCHCRAFT_TEAL,
            ARCH_CYAN,
            ARCH_LIGHT,
            WHITE,
            ARCHCRAFT_TEAL,
            ARCH_CYAN,
        ],
        width,
    }
}
