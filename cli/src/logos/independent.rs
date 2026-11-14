use super::*;

pub(super) fn alpine() -> Logo {
    let lines = vec![
        r##"       .hddddddddddddddddddddddh."##,
        r##"      :dddddddddddddddddddddddddd:"##,
        r##"     /dddddddddddddddddddddddddddd/"##,
        r##"    +dddddddddddddddddddddddddddddd+"##,
        r##"  `sdddddddddddddddddddddddddddddddds`"##,
        r##" `ydddddddddddd++hdddddddddddddddddddy`"##,
        r##".hddddddddddd+`  `+ddddh:-sdddddddddddh."##,
        r##"hdddddddddd+`      `+y:    .sddddddddddh"##,
        r##"ddddddddh+`   `//`   `.`     -sddddddddd"##,
        r##"ddddddh+`   `/hddh/`   `:s-    -sddddddd"##,
        r##"ddddh+`   `/+/dddddh/`   `+s-    -sddddd"##,
        r##"ddd+`   `/o` :dddddddh/`   `oy-    .yddd"##,
        r##"hdddyo+ohddyosdddddddddho+oydddy++ohdddh"##,
        r##".hddddddddddddddddddddddddddddddddddddh."##,
        r##" `yddddddddddddddddddddddddddddddddddy`"##,
        r##"  `sdddddddddddddddddddddddddddddddds`"##,
        r##"    +dddddddddddddddddddddddddddddd+"##,
        r##"     /dddddddddddddddddddddddddddd/"##,
        r##"      :dddddddddddddddddddddddddd:"##,
        r##"       .hddddddddddddddddddddddh."##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: ALPINE_BLUE,
        colors: vec![ALPINE_BLUE, WHITE],
        width,
    }
}

pub(super) fn void() -> Logo {
    let lines = vec![
        r##"                __.;=====;.__"##,
        r##"            _.=+==++=++=+=+===;."##,
        r##"             -=+++=+===+=+=+++++=_"##,
        r##"        .     -=:``     `--==+=++==."##,
        r##"       _vi,    `            --+=++++:"##,
        r##"      .uvnvi.       _._       -==+==+."##,
        r##"     .vvnvnI`    .;==|==;.     :|=||=|."##,
        r##"$2+QmQQm$1pvvnv;$2 _yYsyQQWUUQQQm #QmQ#$1:$2QQQWUV$QQm."##,
        r##" $2-QQWQW$1pvvo$2wZ?.wQQQE$1==<$2QWWQ/QWQW.QQWW$1(:$2 jQWQE"##,
        r##"  $2-$QQQQmmU'  jQQQ$1@+=<$2QWQQ)mQQQ.mQQQC$1+;$2jWQQ@'"##,
        r##"   $2-$WQ8Y$1nI:$2   QWQQwgQQWV$1`$2mWQQ.jQWQQgyyWW@!"##,
        r##"     $1-1vvnvv.     `~+++`        ++|+++"##,
        r##"      +vnvnnv,                 `-|==="##,
        r##"       +vnvnvns.           .      :=-"##,
        r##"        -Invnvvnsi..___..=sv=.     `"##,
        r##"          +Invnvnvnnnnnnnnvvnn;."##,
        r##"            ~|Invnvnvvnvvvnnv}+`"##,
        r##"               -~|{*l}*|~"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: VOID_GREEN,
        colors: vec![VOID_GREEN, WHITE],
        width,
    }
}

pub(super) fn gentoo() -> Logo {
    let lines = vec![
        r##"         -/oyddmdhs+:."##,
        r##"     -o$2dNMMMMMMMMNNmhy+$1-`"##,
        r##"   -y$2NMMMMMMMMMMMNNNmmdhy$1+-"##,
        r##" `o$2mMMMMMMMMMMMMNmdmmmmddhhy$1/`"##,
        r##" om$2MMMMMMMMMMMN$1hhyyyo$2hmdddhhhd$1o`"##,
        r##".y$2dMMMMMMMMMMd$1hs++so/s$2mdddhhhhdm$1+`"##,
        r##" oy$2hdmNMMMMMMMN$1dyooy$2dmddddhhhhyhN$1d."##,
        r##"  :o$2yhhdNNMMMMMMMNNNmmdddhhhhhyym$1Mh"##,
        r##"    .:$2+sydNMMMMMNNNmmmdddhhhhhhmM$1my"##,
        r##"       /m$2MMMMMMNNNmmmdddhhhhhmMNh$1s:"##,
        r##"    `o$2NMMMMMMMNNNmmmddddhhdmMNhs$1+`"##,
        r##"  `s$2NMMMMMMMMNNNmmmdddddmNMmhs$1/."##,
        r##" /N$2MMMMMMMMNNNNmmmdddmNMNdso$1:`"##,
        r##"+M$2MMMMMMNNNNNmmmmdmNMNdso$1/-"##,
        r##"yM$2MNNNNNNNmmmmmNNMmhs+/$1-`"##,
        r##"/h$2MMNNNNNNNNMNdhs++/$1-`"##,
        r##"`/$2ohdmmddhys+++/:$1.`"##,
        r##"  `-//////:--."##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: GENTOO_PURPLE,
        colors: vec![WHITE, GENTOO_PURPLE],
        width,
    }
}

pub(super) fn nixos() -> Logo {
    let lines = vec![
        r##"          $1▗▄▄▄       $2▗▄▄▄▄    ▄▄▄▖"##,
        r##"          $1▜███▙       $2▜███▙  ▟███▛"##,
        r##"           $1▜███▙       $2▜███▙▟███▛"##,
        r##"            $1▜███▙       $2▜██████▛"##,
        r##"     $1▟█████████████████▙ $2▜████▛     $3▟▙"##,
        r##"    $1▟███████████████████▙ $2▜███▙    $3▟██▙"##,
        r##"           $6▄▄▄▄▖           $2▜███▙  $3▟███▛"##,
        r##"          $6▟███▛             $2▜██▛ $3▟███▛"##,
        r##"         $6▟███▛               $2▜▛ $3▟███▛"##,
        r##"$6▟███████████▛                  $3▟██████████▙"##,
        r##"$6▜██████████▛                  $3▟███████████▛"##,
        r##"      $6▟███▛ $5▟▙               $3▟███▛"##,
        r##"     $6▟███▛ $5▟██▙             $3▟███▛"##,
        r##"    $6▟███▛  $5▜███▙           $3▝▀▀▀▀"##,
        r##"    $6▜██▛    $5▜███▙ $4▜██████████████████▛"##,
        r##"     $6▜▛     $5▟████▙ $4▜████████████████▛"##,
        r##"           $5▟██████▙         $4▜███▙"##,
        r##"          $5▟███▛▜███▙         $4▜███▙"##,
        r##"         $5▟███▛  ▜███▙         $4▜███▙"##,
        r##"         $5▝▀▀▀    ▀▀▀▀▘         $4▀▀▀▘"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: NIXOS_CYAN,
        colors: vec![
            NIXOS_BLUE, NIXOS_CYAN, NIXOS_BLUE, NIXOS_CYAN, NIXOS_BLUE, NIXOS_CYAN,
        ],
        width,
    }
}

pub(super) fn linux() -> Logo {
    let lines = vec![
        r##"        $2#####"##,
        r##"       $2#######"##,
        r##"       $2##$1O$2#$1O$2##"##,
        r##"       $2#$3#####$2#"##,
        r##"     $2##$1##$3###$1##$2##"##,
        r##"    $2#$1##########$2##"##,
        r##"   $2#$1############$2##"##,
        r##"   $2#$1############$2###"##,
        r##"  $3##$2#$1###########$2##$3#"##,
        r##"$3######$2#$1#######$2#$3######"##,
        r##"$3#######$2#$1#####$2#$3#######"##,
        r##"  $3#####$2#######$3#####"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: WHITE,
        colors: vec![WHITE, TUX_DARK, TUX_YELLOW],
        width,
    }
}

pub(super) fn freebsd() -> Logo {
    let lines = vec![
        r##"```                        $2`"##,
        r##"  $1` `.....---...$2....--.```   -/"##,
        r##"  $1+o   .--`         $2/y:`      +."##,
        r##"   $1yo`:.            $2:o      `+-"##,
        r##"    $1y/               $2-/`   -o/"##,
        r##"   $1.-                  $2::/sy+:."##,
        r##"   $1/                     $2`--  /"##,
        r##"  $1`:                          $2:`"##,
        r##"  $1`:                          $2:`"##,
        r##"   $1/                          $2/"##,
        r##"   $1.-                        $2-."##,
        r##"    $1--                      $2-."##,
        r##"     $1`:`                  $2`:`"##,
        r##"       .--             `--."##,
        r##"          .---.....----."##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: FREEBSD_RED,
        colors: vec![FREEBSD_WHITE, FREEBSD_RED],
        width,
    }
}

pub(super) fn freebsd_small() -> Logo {
    let lines = vec![
        r##"$1/\,-'''''-,/\"##,
        r##"\_)       (_/"##,
        r##"|           |"##,
        r##"|           |"##,
        r##" ;         ;"##,
        r##"  '-_____-'"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: FREEBSD_RED,
        colors: vec![FREEBSD_RED],
        width,
    }
}

pub(super) fn nixos_small() -> Logo {
    let lines = vec![
        r##"$1  ▗▄   $2▗▄ ▄▖"##,
        r##"$1 ▄▄🬸█▄▄▄$2🬸█▛ $3▃"##,
        r##"$6   ▟▛    $2▜$3▃▟🬕"##,
        r##"$6🬋🬋🬫█      $3█🬛🬋🬋"##,
        r##"$6 🬷▛🮃$5▙    $3▟▛"##,
        r##"$6 🮃 $5▟█🬴$4▀▀▀█🬴▀▀"##,
        r##"  $5▝▀ ▀▘   $4▀▘"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: NIXOS_CYAN,
        colors: vec![
            NIXOS_BLUE, NIXOS_CYAN, NIXOS_BLUE, NIXOS_CYAN, NIXOS_BLUE, NIXOS_CYAN,
        ],
        width,
    }
}

pub(super) fn openbsd() -> Logo {
    let lines = vec![
        r##"$3                                     _"##,
        r##"                                    (_)"##,
        r##"$1              |    ."##,
        r##"$1          .   |L  /|   .         $3 _"##,
        r##"$1      _ . |\ _| \--+._/| .       $3(_)"##,
        r##"$1     / ||\| Y J  )   / |/| ./"##,
        r##"    J  |)'( |        ` F`.'/       $3 _"##,
        r##"$1  -<|  F         __     .-<        $3(_)"##,
        r##"$1    | /       .-'$3. $1`.  /$3-. $1L___"##,
        r##"    J \      <    $3\ $1 | | $5O$3\$1|.-' $3 _"##,
        r##"$1  _J \  .-    \$3/ $5O $3| $1| \  |$1F    $3(_)"##,
        r##"$1 '-F  -<_.     \   .-'  `-' L__"##,
        r##"__J  _   _.     >-'  $1)$4._.   $1|-'"##,
        r##"$1 `-|.'   /_.          $4\_|  $1 F"##,
        r##"  /.-   .                _.<"##,
        r##" /'    /.'             .'  `\"##,
        r##"  /L  /'   |/      _.-'-\"##,
        r##" /'J       ___.---'\|"##,
        r##"   |\  .--' V  | `. `"##,
        r##"   |/`. `-.     `._)"##,
        r##"      / .-.\"##,
        r##"      \ (  `\"##,
        r##"       `.\"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: OPENBSD_YELLOW,
        colors: vec![
            OPENBSD_YELLOW,
            OPENBSD_BLUE,
            WHITE,
            "\x1b[38;2;239;83;80m",
            "\x1b[38;2;38;198;218m",
        ],
        width,
    }
}

pub(super) fn netbsd() -> Logo {
    let lines = vec![
        r##"$1                     `-/oshdmNMNdhyo+:-`"##,
        r##"$2y$1/s+:-``    `.-:+oydNMMMMNhs/-``"##,
        r##"$2-m+$1NMMMMMMMMMMMMMMMMMMMNdhmNMMMmdhs+/-`"##,
        r##" $2-m+$1NMMMMMMMMMMMMMMMMMMMMmy+:`"##,
        r##"  $2-N/$1dMMMMMMMMMMMMMMMds:`"##,
        r##"   $2-N/$1hMMMMMMMMMmho:`"##,
        r##"    $2-N/$1-:/++/:.`"##,
        r##"$2     :M+"##,
        r##"      :Mo"##,
        r##"       :Ms"##,
        r##"        :Ms"##,
        r##"         :Ms"##,
        r##"          :Ms"##,
        r##"           :Ms"##,
        r##"            :Ms"##,
        r##"             :Ms"##,
        r##"              :Ms"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: NETBSD_ORANGE,
        colors: vec![WHITE, NETBSD_ORANGE],
        width,
    }
}

pub(super) fn macos() -> Logo {
    let lines = vec![
        r##"                     ..'"##,
        r##"                 ,xNMM."##,
        r##"               .OMMMMo"##,
        r##"               lMM""##,
        r##"     .;loddo:.  .olloddol;."##,
        r##"   cKMMMMMMMMMMNWMMMMMMMMMM0:"##,
        r##" $2.KMMMMMMMMMMMMMMMMMMMMMMMWd."##,
        r##" XMMMMMMMMMMMMMMMMMMMMMMMX."##,
        r##"$3;MMMMMMMMMMMMMMMMMMMMMMMM:"##,
        r##":MMMMMMMMMMMMMMMMMMMMMMMM:"##,
        r##"$4.MMMMMMMMMMMMMMMMMMMMMMMMX."##,
        r##" kMMMMMMMMMMMMMMMMMMMMMMMMWd."##,
        r##" $5'XMMMMMMMMMMMMMMMMMMMMMMMMMMk"##,
        r##"  'XMMMMMMMMMMMMMMMMMMMMMMMMK."##,
        r##"    $6kMMMMMMMMMMMMMMMMMMMMMMd"##,
        r##"     ;KMMMMMMMWXXWMMMMMMMk."##,
        r##"       "cooc*"    "*coo'""##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: MACOS_GREEN,
        colors: vec![
            MACOS_GREEN,
            MACOS_YELLOW,
            MACOS_ORANGE,
            MACOS_RED,
            MACOS_PURPLE,
            MACOS_BLUE,
        ],
        width,
    }
}

pub(super) fn windows() -> Logo {
    let lines = vec![
        r##"$1        ,.=:!!t3Z3z.,"##,
        r##"       :tt:::tt333EE3"##,
        r##"$1       Et:::ztt33EEEL$2 @Ee.,      ..,"##,
        r##"$1      ;tt:::tt333EE7$2 ;EEEEEEttttt33#"##,
        r##"$1     :Et:::zt333EEQ.$2 $EEEEEttttt33QL"##,
        r##"$1     it::::tt333EEF$2 @EEEEEEttttt33F"##,
        r##"$1    ;3=*^```"*4EEV$2 :EEEEEEttttt33@."##,
        r##"$3    ,.=::::!t=., $1`$2 @EEEEEEtttz33QF"##,
        r##"$3   ;::::::::zt33)$2   "4EEEtttji3P*"##,
        r##"$3  :t::::::::tt33.$4:Z3z..$2  ``$4 ,..g."##,
        r##"$3  i::::::::zt33F$4 AEEEtttt::::ztF"##,
        r##"$3 ;:::::::::t33V$4 ;EEEttttt::::t3"##,
        r##"$3 E::::::::zt33L$4 @EEEtttt::::z3F"##,
        r##"$3{3=*^```"*4E3)$4 ;EEEtttt:::::tZ`"##,
        r##"$3             `$4 :EEEEtttt::::z7"##,
        r##"                 "VEzjt:;;z>*`"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: WINDOWS_BLUE,
        colors: vec![WINDOWS_RED, WINDOWS_GREEN, WINDOWS_BLUE, WINDOWS_YELLOW],
        width,
    }
}

pub(super) fn cat_mascot() -> Logo {
    let lines = vec![
        r##"       $1|\      _,,,---,,_"##,
        r##" $2ZZZzz $1/,`.-'`'    -.  ;-;;,_"##,
        r##"      $1|,4-  ) )-,_. ,\ (  `'-'"##,
        r##"     $2'---''(_/--'  `-'\_)"##,
        r##""##,
        r##"        $1/\_/\   $2.---."##,
        r##"       $1( $3o$1.$3o$1 ) $2/  $1RustFetch"##,
        r##"        $1> ^ <  $2'---'"##,
        r##"       $1/|   |\"##,
        r##"      $1(_|   |_)"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: CAT_AMBER,
        colors: vec![CAT_AMBER, CAT_CREAM, CAT_EMERALD],
        width,
    }
}

pub(super) fn bengal_cat() -> Logo {
    let lines = vec![
        r##"         $1/\_____/\"##,
        r##"        $1/  $3o   o  $1\"##,
        r##"       $1( ==  $2^  $1== )"##,
        r##"        $1)  $2~   ~  $1("##,
        r##"       $1(  $2(  .  )  $1)"##,
        r##"      $1( (  )   (  ) )"##,
        r##"     $1(__(__)___(__)__)"##,
        r##"        $2"Bengal" v0.1.2"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: CAT_AMBER,
        colors: vec![CAT_AMBER, UBUNTU_ORANGE, CAT_EMERALD],
        width,
    }
}

pub(super) fn maine_coon_cat() -> Logo {
    let lines = vec![
        r##"         $1/\_ _ _/\"##,
        r##"        $1/  $3>   <  $1\"##,
        r##"       $1( ==  $2*  $1== )"##,
        r##"        $1)  $2~   ~  $1("##,
        r##"       $1(  $2/|   |\ $1)"##,
        r##"      $1( (  )   (  ) )"##,
        r##"     $1(__(__)___(__)__)"##,
        r##"      $2"Maine Coon" v0.1.3"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: CAT_AMBER,
        colors: vec![CAT_AMBER, UBUNTU_ORANGE, CAT_EMERALD],
        width,
    }
}

pub(super) fn siamese_cat() -> Logo {
    let lines = vec![
        r##"         $1/\_____/\"##,
        r##"        $1/  $3*   *  $1\"##,
        r##"       $1( ==  $2^  $1== )"##,
        r##"        $1)   $2.-.   $1("##,
        r##"       $1(   $2/   \   $1)"##,
        r##"      $1( (  )   (  ) )"##,
        r##"     $1(__(__)___(__)__)"##,
        r##"       $2"Siamese" v0.1.4"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: CAT_CREAM,
        colors: vec![CAT_CREAM, UBUNTU_ORANGE, ARCH_LIGHT],
        width,
    }
}

pub(super) fn ragdoll_cat() -> Logo {
    let lines = vec![
        r##"         $1/\_____/\"##,
        r##"        $1/  $3*   *  $1\"##,
        r##"       $1( ==  $2^  $1== )"##,
        r##"        $1)   $2.-.   $1("##,
        r##"       $1(   $2/   \   $1)"##,
        r##"      $1( (  )   (  ) )"##,
        r##"     $1(__(__)___(__)__)"##,
        r##"       $2"Ragdoll" v0.1.5"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: CAT_CREAM,
        colors: vec![CAT_CREAM, UBUNTU_ORANGE, ARCH_LIGHT],
        width,
    }
}

pub(super) fn turkish_van() -> Logo {
    let lines = vec![
        r##"         $3/\_____/\"##,
        r##"        $1/  $2o   $3o  $1\"##,
        r##"       $1( ==  ^  == )"##,
        r##"        $1)         ("##,
        r##"       $1(           )"##,
        r##"      $1( (  )   (  ) )"##,
        r##"     $1(__(__)___(__)__)"##,
        r##"       $2"Turkish Van""##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: WHITE,
        colors: vec![WHITE, ARCH_LIGHT, CAT_AMBER],
        width,
    }
}

pub(super) fn turkish_angora() -> Logo {
    let lines = vec![
        r##"         $1/\_____/\"##,
        r##"        $1/  $2o   o  $1\"##,
        r##"       $1( ==  $3^  $1== )"##,
        r##"        $1)  ~   ~  ("##,
        r##"       $1(           )"##,
        r##"      $1( (  )   (  ) )"##,
        r##"     $1(__(__)___(__)__)"##,
        r##"      $2"Turkish Angora""##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: CAT_CREAM,
        colors: vec![WHITE, NIXOS_CYAN, CAT_CREAM],
        width,
    }
}

pub(super) fn cat_small() -> Logo {
    let lines = vec![r##" $1/\_/\"##, r##"$1( $2o$1.$2o$1 )"##, r##" $1> ^ <"##];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: CAT_AMBER,
        colors: vec![CAT_AMBER, CAT_EMERALD],
        width,
    }
}
