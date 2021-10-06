package anotherone;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

public class Lib {

    private static final Logger log = LogManager.getLogger(Lib.class);

    public static void doThing() {
        log.info("Working !");
    }
}