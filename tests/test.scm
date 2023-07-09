(add-to-load-path ".")
(use-modules (srfi srfi-64)
             (bats))

;; TODO: Consider using test-group-with-cleanup to simplify tests.

(test-begin "make-track!-makes-a-track")
(test-equal 0 (length (tracks)))
(make-track!)
(test-equal 1 (length (tracks)))
(delete-all-tracks!)
(test-end)

(test-begin "delete-all-tracks!-deletes-all-tracks")
(while (< (length (track-ids)) 10)
  (make-track!))
(test-equal 10 (length (track-ids)))
(delete-all-tracks!)
(test-equal 0 (length (track-ids)))
(test-end)

(test-begin "make-track!-supports-plugin-instantiation")
(define test-plugin-id '(lv2 . "http://drobilla.net/plugins/mda/EPiano"))
(define test-track-id (make-track! #:plugin-ids (list test-plugin-id)))
(define test-track (track test-track-id))
(define test-plugin-instance-ids (assoc-ref test-track 'plugin-instance-ids))
(define test-plugin-instances (map plugin-instance test-plugin-instance-ids))
(test-equal '((lv2 . "http://drobilla.net/plugins/mda/EPiano"))
  (map (lambda (plugin-instance) (assoc-ref plugin-instance 'plugin-id))
       test-plugin-instances))
(delete-all-tracks!)
(test-end)

(test-begin "make-plugin-instance!-can-instantiate-plugins")
(define test-track-id (make-track!))
(test-equal '()
  (assoc-ref (track test-track-id) 'plugin-instance-ids))
(make-plugin-instance! test-track-id
                       '(lv2 . "http://drobilla.net/plugins/mda/Piano"))
(make-plugin-instance! test-track-id
                       '(lv2 . "http://drobilla.net/plugins/mda/EPiano"))
(define test-plugin-instance-ids (map plugin-instance
                                      (assoc-ref (track test-track-id)
                                                 'plugin-instance-ids)))
(define test-plugin-ids (map (lambda (p) (assoc-ref p 'plugin-id))
                             test-plugin-instance-ids))
(test-equal test-plugin-ids
  '((lv2 . "http://drobilla.net/plugins/mda/Piano")
    (lv2 . "http://drobilla.net/plugins/mda/EPiano")))
(delete-all-tracks!)
(test-end)

(test-begin "test-plugins-have-metadata")
(define test-plugin (car (plugins)))
(test-assert (list? test-plugin))
(test-equal (map car test-plugin)
  '(classes plugin-id name instrument?))
(test-end)
