use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref FORMAT_PATTERNS: HashMap<&'static str, &'static str> = {
        HashMap::from([
            (
                "date-time",
                r"(?P<date>[0-9]{4}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12][0-9]|3[01]))[tT](?P<time>(?:[01][0-9]|2[0-3]):[0-5][0-9]:(?:[0-5][0-9]|60)(?P<time_fraction>\.[0-9]+)?(?P<time_zone>[zZ]|[+-](?:[01][0-9]|2[0-3]):[0-5][0-9]))",
            ),
            (
                "time",
                r"(?:[01][0-9]|2[0-3]):[0-5][0-9]:(?:[0-5][0-9]|60)(?P<time_fraction>\.[0-9]+)?(?P<time_zone>[zZ]|[+-](?:[01][0-9]|2[0-3]):[0-5][0-9])",
            ),
            (
                "date",
                r"[0-9]{4}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12][0-9]|3[01])",
            ),
            (
                "duration",
                r"P(?:(?P<dur_date>(?:(?P<dur_year>[0-9]+Y(?:[0-9]+M(?:[0-9]+D)?)?)|(?P<dur_month>[0-9]+M(?:[0-9]+D)?)|(?P<dur_day>[0-9]+D))(?:T(?:(?P<dur_hour>[0-9]+H(?:[0-9]+M(?:[0-9]+S)?)?)|(?P<dur_minute>[0-9]+M(?:[0-9]+S)?)|(?P<dur_second>[0-9]+S)))?)|(?P<dur_time>T(?:(?P<dur_hour2>[0-9]+H(?:[0-9]+M(?:[0-9]+S)?)?)|(?P<dur_minute2>[0-9]+M(?:[0-9]+S)?)|(?P<dur_second2>[0-9]+S)))|(?P<dur_week>[0-9]+W))",
            ),
            (
                "email",
                r"(?P<local_part>(?P<dot_string>[^\s@\.]+(\.[^\s@\.]+)*))@((?P<domain>(?P<sub_domain>[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?)(\.(?P<sub_domain2>[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?))*)|\[(?P<ipv4>((([0-9])|(([1-9])[0-9]|(25[0-5]|(2[0-4]|(1)[0-9])[0-9])))\.){3}(([0-9])|(([1-9])[0-9]|(25[0-5]|(2[0-4]|(1)[0-9])[0-9]))))\])",
            ),
            (
                "hostname",
                r"[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*",
            ),
            (
                "ipv4",
                r"((([0-9])|(([1-9])[0-9]|(25[0-5]|(2[0-4]|(1)[0-9])[0-9])))\.){3}(([0-9])|(([1-9])[0-9]|(25[0-5]|(2[0-4]|(1)[0-9])[0-9])))",
            ),
            (
                "ipv6",
                r"(?:(?P<full>(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}))|(?:::(?:[0-9a-fA-F]{1,4}:){0,5}(?P<ls32>[0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}))|(?:(?P<h16_1>[0-9a-fA-F]{1,4})?::(?:[0-9a-fA-F]{1,4}:){0,4}(?P<ls32_1>[0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}))|(?:((?:[0-9a-fA-F]{1,4}:){0,1}[0-9a-fA-F]{1,4})?::(?:[0-9a-fA-F]{1,4}:){0,3}(?P<ls32_2>[0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}))|(?:((?:[0-9a-fA-F]{1,4}:){0,2}[0-9a-fA-F]{1,4})?::(?:[0-9a-fA-F]{1,4}:){0,2}(?P<ls32_3>[0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}))|(?:((?:[0-9a-fA-F]{1,4}:){0,3}[0-9a-fA-F]{1,4})?::[0-9a-fA-F]{1,4}:(?P<ls32_4>[0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}))|(?:((?:[0-9a-fA-F]{1,4}:){0,4}[0-9a-fA-F]{1,4})?::(?P<ls32_5>[0-9a-fA-F]{1,4}:[0-9a-fA-F]{1,4}))|(?:((?:[0-9a-fA-F]{1,4}:){0,5}[0-9a-fA-F]{1,4})?::(?P<h16_2>[0-9a-fA-F]{1,4}))|(?:((?:[0-9a-fA-F]{1,4}:){0,6}[0-9a-fA-F]{1,4})?::)",
            ),
            (
                "uuid",
                r"(?P<time_low>[0-9a-fA-F]{8})-(?P<time_mid>[0-9a-fA-F]{4})-(?P<time_high_and_version>[0-9a-fA-F]{4})-(?P<clock_seq_and_reserved>[0-9a-fA-F]{2})(?P<clock_seq_low>[0-9a-fA-F]{2})-(?P<node>[0-9a-fA-F]{12})",
            ),
            ("unknown", r"(?s:.*)"),
        ])
    };
}

pub fn lookup_format(name: &str) -> Option<&str> {
    FORMAT_PATTERNS.get(name).copied()
}
